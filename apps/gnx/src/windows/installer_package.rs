use super::release_source::{self, wide};
use crate::protocol::invalid;
use std::{io, path::Path, ptr::null};
use windows_sys::Win32::{
    Foundation::ERROR_NO_MORE_ITEMS,
    System::{ApplicationInstallationAndServicing::*, Registry::*},
};

const UPGRADE: &str = "{7A34F3DC-EA24-4DDF-8ADB-5817BF28BD21}";
const BUNDLE: &str = "{E7836E76-E93D-47B6-9B38-9835BBFCE98D}";

struct Msi(u32);
impl Drop for Msi {
    fn drop(&mut self) {
        unsafe {
            MsiCloseHandle(self.0);
        }
    }
}
fn checked(code: u32) -> io::Result<()> {
    if code == 0 {
        Ok(())
    } else {
        Err(io::Error::from_raw_os_error(code as i32))
    }
}
fn property(database: &Msi, name: &str) -> io::Result<String> {
    // Names are fixed by the caller, never supplied by a release or the CLI.
    let mut view = 0;
    checked(unsafe {
        MsiDatabaseOpenViewW(
            database.0,
            wide(&format!(
                "SELECT `Value` FROM `Property` WHERE `Property`='{name}'"
            ))
            .as_ptr(),
            &mut view,
        )
    })?;
    let view = Msi(view);
    checked(unsafe { MsiViewExecute(view.0, 0) })?;
    let mut record = 0;
    checked(unsafe { MsiViewFetch(view.0, &mut record) })?;
    let record = Msi(record);
    let mut value = [0u16; 1024];
    let mut count = value.len() as u32;
    checked(unsafe { MsiRecordGetStringW(record.0, 1, value.as_mut_ptr(), &mut count) })?;
    let mut extra = 0;
    if unsafe { MsiViewFetch(view.0, &mut extra) } != ERROR_NO_MORE_ITEMS {
        if extra != 0 {
            drop(Msi(extra));
        }
        return Err(invalid("propiedad MSI ambigua"));
    }
    String::from_utf16(&value[..count as usize]).map_err(|_| invalid("propiedad MSI inválida"))
}
pub(super) fn inspect(path: &Path, expected_version: &str) -> io::Result<()> {
    let text = path.to_str().ok_or_else(|| invalid("ruta MSI inválida"))?;
    let mut database = 0;
    // Null MSIDBOPEN_READONLY: inspect without installing or running custom actions.
    checked(unsafe { MsiOpenDatabaseW(wide(text).as_ptr(), null(), &mut database) })?;
    let database = Msi(database);
    if !property(&database, "UpgradeCode")?.eq_ignore_ascii_case(UPGRADE)
        || property(&database, "ProductName")? != "GnX Core"
        || property(&database, "Manufacturer")? != "GnX"
        || property(&database, "ProductVersion")? != expected_version
        || property(&database, "ALLUSERS")? != "1"
    {
        return Err(invalid("identidad o versión MSI no coincide con GnX"));
    }
    release_source::version(expected_version)?;
    Ok(())
}
fn installed_property(product: &[u16], name: &str) -> io::Result<String> {
    let mut result = [0u16; 32768];
    let mut length = result.len() as u32;
    checked(unsafe {
        MsiGetProductInfoW(
            product.as_ptr(),
            wide(name).as_ptr(),
            result.as_mut_ptr(),
            &mut length,
        )
    })?;
    String::from_utf16(&result[..length as usize]).map_err(|_| invalid("registro MSI inválido"))
}

// Native upgrade identity, not display-name or folder-name adoption.
pub(super) fn installed() -> io::Result<String> {
    let mut product = [0u16; 39];
    checked(unsafe {
        MsiEnumRelatedProductsW(wide(UPGRADE).as_ptr(), 0, 0, product.as_mut_ptr())
    })?;
    let mut extra = [0u16; 39];
    if unsafe { MsiEnumRelatedProductsW(wide(UPGRADE).as_ptr(), 0, 1, extra.as_mut_ptr()) }
        != ERROR_NO_MORE_ITEMS
    {
        return Err(invalid(
            "más de un MSI relacionado; mantenimiento nativo requerido",
        ));
    }
    let version = installed_property(&product, "VersionString")?;
    let cache = installed_property(&product, "LocalPackage")?;
    inspect(Path::new(&cache), &version)?;
    Ok(version)
}

pub(super) fn uses_bundle() -> io::Result<bool> {
    let mut found = 0;
    let installed_version = installed()?;
    for prefix in [
        r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
        r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
    ] {
        let mut key = std::ptr::null_mut();
        let code = unsafe {
            RegOpenKeyExW(
                HKEY_LOCAL_MACHINE,
                wide(prefix).as_ptr(),
                0,
                KEY_READ | KEY_WOW64_64KEY,
                &mut key,
            )
        };
        if code == 2 {
            continue;
        }
        checked(code)?;
        let result = (|| {
            for index in 0..4096 {
                let mut name = [0u16; 256];
                let mut length = name.len() as u32;
                let code = unsafe {
                    RegEnumKeyExW(
                        key,
                        index,
                        name.as_mut_ptr(),
                        &mut length,
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                        std::ptr::null_mut(),
                    )
                };
                if code == ERROR_NO_MORE_ITEMS {
                    return Ok(());
                }
                checked(code)?;
                let name = String::from_utf16(&name[..length as usize])
                    .map_err(|_| invalid("registro Burn inválido"))?;
                let child = format!("{prefix}\\{name}");
                if release_source::machine(&child, "BundleUpgradeCode")?
                    .is_some_and(|codes| codes.split('\0').any(|s| s.eq_ignore_ascii_case(BUNDLE)))
                {
                    let version = release_source::machine(&child, "BundleVersion")?
                        .ok_or_else(|| invalid("versión Burn ausente"))?;
                    // Burn serializes a fourth, zero revision. Product releases use three.
                    let version = version
                        .strip_suffix(".0")
                        .filter(|v| v.split('.').count() == 3)
                        .unwrap_or(&version);
                    release_source::version(version)?;
                    if version != installed_version {
                        return Err(invalid(
                            "MSI y bundle tienen versiones distintas; resolver mantenimiento nativo antes de actualizar",
                        ));
                    }
                    found += 1;
                }
            }
            Err(invalid("inventario Burn excede el límite"))
        })();
        unsafe {
            RegCloseKey(key);
        }
        result?;
    }
    if found > 1 {
        return Err(invalid(
            "varios bundles relacionados; mantenimiento nativo requerido",
        ));
    }
    if found == 0 {
        let mut product = [0u16; 39];
        checked(unsafe {
            MsiEnumRelatedProductsW(wide(UPGRADE).as_ptr(), 0, 0, product.as_mut_ptr())
        })?;
        let product =
            String::from_utf16(&product[..38]).map_err(|_| invalid("código MSI inválido"))?;
        for prefix in [
            r"SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall",
            r"SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall",
        ] {
            let mut hidden = 0u32;
            let mut size = 4;
            let code = unsafe {
                RegGetValueW(
                    HKEY_LOCAL_MACHINE,
                    wide(&format!("{prefix}\\{product}")).as_ptr(),
                    wide("SystemComponent").as_ptr(),
                    RRF_RT_REG_DWORD | RRF_SUBKEY_WOW6464KEY,
                    std::ptr::null_mut(),
                    (&mut hidden as *mut u32).cast(),
                    &mut size,
                )
            };
            if code != 0 && code != 2 {
                checked(code)?;
            }
            if code == 0 && hidden != 0 {
                return Err(invalid(
                    "MSI oculto sin bundle verificado; no se cambia su ownership",
                ));
            }
        }
    }
    Ok(found == 1)
}
