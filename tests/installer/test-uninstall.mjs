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

assert.match(product, /Version="0\.1\.18\.0"/);
assert.match(bundle, /Version="0\.1\.18\.0"/);
assert.match(product, /UpgradeCode="5EAA2C0C-1E4C-4E4D-89E5-3D6F1A7E4C11"/);
assert.match(bundle, /UpgradeCode="DB8BFB10-FA20-4F4F-9B3E-35DBD15A2A1A"/);
assert.match(bundle, /LaunchTarget="\[ProgramFilesFolder\]GnX Node\\gnx\.exe"/);
assert.match(bundle, /LaunchTargetElevatedId="GnxSetupElevated"/);
assert.match(bundle, /ApprovedExeForElevation[^>]*Id="GnxSetupElevated"[^>]*Key="SOFTWARE\\GnX\\Node"[^>]*Value="SetupPath"[^>]*Bitness="always32"/);
assert.match(product, /RegistryValue[^>]*Root="HKLM"[^>]*Key="SOFTWARE\\GnX\\Node"[^>]*Name="SetupPath"[^>]*Value="\[#SetupExe\]"/);
assert.match(product, /RemoveFolderEx[^>]*Id="RemoveLegacyWebViewData"[^>]*On="install"/);
assert.match(product, /LEGACYWEBVIEWCACHE[^>]*gnx-setup\.exe\.WebView2/);
assert.match(product, /Environment[^>]*Id="GnxCliHostPath"[^>]*Name="PATH"[^>]*Value="\[INSTALLFOLDER\]"[^>]*Part="last"[^>]*Action="set"[^>]*System="yes"/);
assert.match(bundle, /LaunchArguments="--pause"/);
assert.doesNotMatch(bundle, /WebView2|RegistrySearch|EdgeUpdate/i);
assert.doesNotMatch(await read('installer/bundle/Bundle.wixproj'), /WixToolset\.Util/i);
assert.match(wixproj, /target\\release\\gnx\.exe/);
assert.doesNotMatch(wixproj, /target\\release\\gnx-setup\.exe/);
assert.doesNotMatch(wixproj, /SetupUi|apps\\setup\\ui/);
assert.doesNotMatch(buildScript, /apps\\setup\\ui|Copy-Item.*SetupUi/);
assert.match(readme, /silent installs do not prompt/i);
assert.match(readme, /Start-Process -Verb RunAs -Wait/);


// MSI owns service lifecycle and invokes the fixed host binary for complete
// product cleanup. Upgrade removal must never run destructive cleanup.
assert.match(product, /ServiceInstall[^>]*Name="GnXHostAgent"[^>]*Start="auto"/);
assert.match(product, /ServiceControl[^>]*Name="GnXHostAgent"[^>]*Stop="both"[^>]*Remove="uninstall"/);
assert.match(product, /ComponentGroup Id="HostPayloadFiles"/);
assert.match(product, /ComponentGroup Id="NodePayloadFiles"/);
assert.match(product, /CustomAction Id="CleanupFreshResidue"[^>]*FileRef="HostAgentExe"[^>]*ExeCommand="--cleanup"[^>]*Execute="deferred"[^>]*Impersonate="no"[^>]*Return="check"/);
assert.match(product, /CustomAction Id="CleanupOnUninstall"[^>]*FileRef="HostAgentExe"[^>]*ExeCommand="--cleanup"[^>]*Execute="deferred"[^>]*Impersonate="no"[^>]*Return="check"/);
assert.match(product, /Custom Action="CleanupFreshResidue"[^>]*After="InstallFiles"[^>]*Condition="NOT Installed AND NOT WIX_UPGRADE_DETECTED"/);
assert.match(product, /Custom Action="CleanupOnUninstall"[^>]*Before="RemoveFiles"[^>]*Condition="REMOVE~=&quot;ALL&quot; AND NOT UPGRADINGPRODUCTCODE"/);

// Burn remains a thin wrapper over the MSI; cleanup has one owner.
assert.match(bundle, /<MsiPackage\s+SourceFile="\$\(var\.MsiPackage\)"[^>]*Visible="no"/);
assert.doesNotMatch(bundle, /Deprovision|CustomAction|ExeCommand/i);

for (const term of [
  'WSL node',
  'runtime data',
  'private CA',
  'hosts',
  'port-proxy',
  'credentials',
]) assert.match(readme, new RegExp(term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i'), `README must document ${term}`);
assert.match(readme, /Complete uninstall acceptance checklist/);
assert.match(readme, /Upgrade and reinstallation contract/);
for (const check of [
  'GnXHostAgent.*absent',
  'installed Windows files.*absent',
  'WSL.*absent',
  'state.*absent',
  'hosts.*port-proxy.*absent',
]) assert.match(readme, new RegExp(check, 'i'), `missing uninstall check: ${check}`);

const cleanup = await read('apps/host-agent/src/cleanup.rs');
assert.match(cleanup, /host_access::cleanup\(&state_dir\)\?/);
assert.ok(cleanup.indexOf('host_access::cleanup(&state_dir)?;') < cleanup.indexOf('std::fs::remove_dir_all(&state_root)?;'), 'host access cleanup must run before state deletion');
assert.match(cleanup, /listenaddress=127\.0\.0\.1/);
assert.match(cleanup, /listenport=443/);
const dedicatedAccount = await read('apps/host-agent/src/dedicated_account.rs');
assert.match(dedicatedAccount, /"--unregister"\.into\(\)[\s\S]*linux_node::DISTRO_NAME/);
assert.match(dedicatedAccount, /remove_dir_all\(&profile\)/);
assert.doesNotMatch(dedicatedAccount, /["']--shutdown["']/i);

const linuxNode = await read('apps/host-agent/src/linux_node.rs');
assert.match(linuxNode, /if !distro_is_registered\(\)/);
assert.match(linuxNode, /materialize_payload\(&node_root\)/);

console.log('installer uninstall contract checks passed');
