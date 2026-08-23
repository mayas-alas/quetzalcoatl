#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RuntimeOperation {
    Ping,
    PveClusterPrepare,
    PveClusterVerifyNode,
    PveClusterCreate,
    PveClusterVerify,
    PveClusterJoin,
    PveClusterConfirmMember,
    PveConfigure,
    TailscalePrepare,
    TailscaleRename,
    PlatformReconcile,
    PlatformDeploy,
    ForgejoAdminShow,
    ForgejoAdminReset,
}

impl RuntimeOperation {
    pub(crate) const fn argv(self) -> &'static [&'static str] {
        match self {
            Self::Ping => &["ping"],
            Self::PveClusterPrepare => &["pve-cluster-create", "prepare"],
            Self::PveClusterVerifyNode => &["pve-cluster-create", "verify-node"],
            Self::PveClusterCreate => &["pve-cluster-create", "create"],
            Self::PveClusterVerify => &["pve-cluster-create", "verify"],
            Self::PveClusterJoin => &["pve-cluster-create", "join"],
            Self::PveClusterConfirmMember => &["pve-cluster-create", "confirm-member"],
            Self::PveConfigure => &["pve-configure"],
            Self::TailscalePrepare => &["tailscale-prepare"],
            Self::TailscaleRename => &["tailscale-rename"],
            Self::PlatformReconcile => &["platform-reconcile"],
            Self::PlatformDeploy => &["platform-deploy"],
            Self::ForgejoAdminShow => &["forgejo-admin", "show"],
            Self::ForgejoAdminReset => &["forgejo-admin", "reset"],
        }
    }
}
