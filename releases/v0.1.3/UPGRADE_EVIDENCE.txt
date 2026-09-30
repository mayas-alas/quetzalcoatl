GnX Node v0.1.3 upgrade acceptance

Burn exit code: 0
Detected related bundle: type=Upgrade, version=0.1.2.0
Detected related MSI: operation=MajorUpgrade, version=0.1.2.0
Planned new GnxNode.msi: execute=Install
Planned related v0.1.2 bundle: execute=Uninstall
Applied new GnxNode.msi: result=0x0, ARPSYSTEMCOMPONENT=1

Post-upgrade:
- GnXHostAgent: Running
- gnx-setup resolved from host system PATH
- gnx-setup version: 0.1.3
- Visible Installed Apps entries: 1
- Visible bundle version: 0.1.3.0
- Obsolete WebView2 profile: absent
- app.gnx: HTTP 200
- compute.gnx: HTTP 200

No onboarding credential was requested or changed during the upgrade.
The full machine-specific Burn/MSI logs remain under build/ and are not shipped.
