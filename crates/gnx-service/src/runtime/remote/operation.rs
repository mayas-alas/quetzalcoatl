#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::runtime) enum RuntimeOperation {
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
}

impl RuntimeOperation {
    pub(in crate::runtime) const fn argv(self) -> &'static [&'static str] {
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
        }
    }
}
