import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

// Deliberately compact: one static contract covers the MSI, Burn delegation,
// and the intentionally limited uninstall boundary without mutating a host.
const root = join(dirname(fileURLToPath(import.meta.url)), '..', '..');
const read = relative => readFile(join(root, relative), 'utf8');

const [product, bundle, wixproj, buildScript, readme] = await Promise.all([
  read('installer/Product.wxs'),
  read('installer/bundle/Bundle.wxs'),
  read('installer/GnxNode.wixproj'),
  read('installer/scripts/build.ps1'),
  read('installer/README.md'),
]);

assert.match(product, /Version="0\.1\.3\.0"/);
assert.match(bundle, /Version="0\.1\.3\.0"/);
assert.match(product, /UpgradeCode="5EAA2C0C-1E4C-4E4D-89E5-3D6F1A7E4C11"/);
assert.match(bundle, /UpgradeCode="DB8BFB10-FA20-4F4F-9B3E-35DBD15A2A1A"/);
assert.match(bundle, /LaunchTarget="\[ProgramFilesFolder\]GnX Node\\gnx-setup\.exe"/);
assert.match(bundle, /LaunchTargetElevatedId="GnxSetupElevated"/);
assert.match(bundle, /ApprovedExeForElevation[^>]*Id="GnxSetupElevated"[^>]*Key="SOFTWARE\\GnX\\Node"[^>]*Value="SetupPath"[^>]*Bitness="always32"/);
assert.match(product, /RegistryValue[^>]*Root="HKLM"[^>]*Key="SOFTWARE\\GnX\\Node"[^>]*Name="SetupPath"[^>]*Value="\[#SetupExe\]"/);
assert.match(product, /RemoveFolderEx[^>]*Id="RemoveLegacyWebViewData"[^>]*On="install"/);
assert.match(product, /LEGACYWEBVIEWCACHE[^>]*gnx-setup\.exe\.WebView2/);
assert.match(product, /Environment[^>]*Id="GnxSetupHostPath"[^>]*Name="PATH"[^>]*Value="\[INSTALLFOLDER\]"[^>]*Part="last"[^>]*Action="set"[^>]*System="yes"/);
assert.match(bundle, /LaunchArguments="--pause"/);
assert.doesNotMatch(bundle, /WebView2|RegistrySearch|EdgeUpdate/i);
assert.doesNotMatch(await read('installer/bundle/Bundle.wixproj'), /WixToolset\.Util/i);
assert.doesNotMatch(wixproj, /SetupUi|apps\\setup\\ui/);
assert.doesNotMatch(buildScript, /apps\\setup\\ui|Copy-Item.*SetupUi/);
assert.match(readme, /silent installs do not prompt/i);
assert.match(readme, /Start-Process -Verb RunAs -Wait/);


// MSI owns only the Windows package: service lifecycle plus installed payload.
assert.match(product, /ServiceInstall[^>]*Name="GnXHostAgent"[^>]*Start="auto"/);
assert.match(product, /ServiceControl[^>]*Name="GnXHostAgent"[^>]*Stop="both"[^>]*Remove="uninstall"/);
assert.match(product, /ComponentGroup Id="HostPayloadFiles"/);
assert.match(product, /ComponentGroup Id="NodePayloadFiles"/);
assert.doesNotMatch(product, /ProgramData|CustomAction|RemoveFile/i, 'MSI must not own runtime state cleanup');

// Burn must remain a thin wrapper over that same MSI, including uninstall.
assert.match(bundle, /<MsiPackage\s+SourceFile="\$\(var\.MsiPackage\)"[^>]*Visible="no"/);
assert.doesNotMatch(bundle, /Deprovision|CustomAction|ExeCommand/i);

// The boundary is explicit, including sensitive residual state and network
// leftovers, so a future cleanup change cannot silently become misleading.
for (const term of [
  'WSL node',
  'runtime data',
  'private CA',
  'hosts',
  'port-proxy',
  'credentials',
]) assert.match(readme, new RegExp(term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i'), `README must document ${term}`);
assert.match(readme, /no fake `Deprovision`\s+action/i);

// One operator-level smoke checklist is enough for this stage; it must not
// imply that uninstall removes the Linux runtime.
assert.match(readme, /Minimal uninstall acceptance checklist/);
assert.match(readme, /Upgrade and reinstallation contract/);
for (const check of [
  'GnXHostAgent.*absent',
  'installed Windows files.*absent',
  'WSL.*retained',
  'state.*retained',
  'hosts.*port-proxy.*retained',
]) assert.match(readme, new RegExp(check, 'i'), `missing uninstall check: ${check}`);

const linuxNode = await read('apps/host-agent/src/linux_node.rs');
assert.match(linuxNode, /if !distro_is_registered\(\)/);
assert.match(linuxNode, /materialize_payload\(&node_root\)/);

console.log('installer uninstall contract checks passed');
