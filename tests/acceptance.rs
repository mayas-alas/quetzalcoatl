use gnx::{
    app,
    config::Config,
    port::{host::Host, runtime::Runtime, state::StateStore},
    report::Capability,
};
use std::cell::RefCell;
struct Fake {
    events: RefCell<Vec<&'static str>>,
    healthy: bool,
}
impl Host for Fake {
    fn prerequisites(&self) -> Result<(), String> {
        Ok(())
    }
}
impl Runtime for Fake {
    fn observe(&self) -> Vec<Capability> {
        ["access", "control", "compute"]
            .iter()
            .map(|n| Capability {
                name: n.to_string(),
                healthy: self.healthy,
                code: "TEST".into(),
            })
            .collect()
    }
    fn reconcile(&self, _: &Config) -> Result<(), String> {
        self.events.borrow_mut().push("reconcile");
        Ok(())
    }
}
impl StateStore for Fake {
    fn current(&self) -> Result<Option<String>, String> {
        Ok(Some("previous".into()))
    }
    fn stage(&self, _: &Config, _: &str) -> Result<(), String> {
        self.events.borrow_mut().push("stage");
        Ok(())
    }
    fn promote(&self) -> Result<(), String> {
        self.events.borrow_mut().push("promote");
        Ok(())
    }
}
#[test]
fn plan_does_not_mutate() {
    let f = Fake {
        events: RefCell::new(vec![]),
        healthy: false,
    };
    let c = Config::parse(include_str!("../gnx.toml")).unwrap();
    app::execute("plan", &c, &f, &f, &f);
    app::execute("plan", &c, &f, &f, &f);
    assert!(f.events.borrow().is_empty());
}

struct ReleaseChange {
    desired: String,
    reconciled: RefCell<bool>,
}
impl Host for ReleaseChange {
    fn prerequisites(&self) -> Result<(), String> {
        Ok(())
    }
}
impl Runtime for ReleaseChange {
    fn revision(&self, _: &Config) -> String {
        self.desired.clone()
    }
    fn observe(&self) -> Vec<Capability> {
        ["access", "control", "compute"]
            .iter()
            .map(|name| Capability {
                name: (*name).into(),
                healthy: true,
                code: "OK".into(),
            })
            .collect()
    }
    fn reconcile(&self, _: &Config) -> Result<(), String> {
        self.reconciled.replace(true);
        Ok(())
    }
}
struct ReleaseState {
    current: String,
    staged: RefCell<Option<String>>,
}
impl StateStore for ReleaseState {
    fn current(&self) -> Result<Option<String>, String> {
        Ok(Some(self.current.clone()))
    }
    fn stage(&self, _: &Config, revision: &str) -> Result<(), String> {
        self.staged.replace(Some(revision.into()));
        Ok(())
    }
    fn promote(&self) -> Result<(), String> {
        Ok(())
    }
}
#[test]
fn authenticated_release_change_forces_reconciliation_with_unchanged_intent() {
    let config = Config::parse(include_str!("../gnx.toml")).unwrap();
    let desired = "a".repeat(64);
    let runtime = ReleaseChange {
        desired: desired.clone(),
        reconciled: RefCell::new(false),
    };
    let state = ReleaseState {
        current: config.revision(),
        staged: RefCell::new(None),
    };
    let report = app::apply::run(
        gnx::report::Report::new("apply", gnx::report::State::ActionRequired, "TEST", None),
        &config,
        &runtime,
        &runtime,
        &state,
    );
    assert_eq!(report.revision, Some(desired.clone()));
    assert_eq!(*state.staged.borrow(), Some(desired));
    assert!(*runtime.reconciled.borrow());
}
#[test]
fn unhealthy_candidate_preserves_last_valid() {
    let f = Fake {
        events: RefCell::new(vec![]),
        healthy: false,
    };
    let c = Config::parse(include_str!("../gnx.toml")).unwrap();
    let r = app::execute("apply", &c, &f, &f, &f);
    assert_ne!(r.exit(), 0);
    assert_eq!(r.revision.as_deref(), Some("previous"));
    assert_eq!(*f.events.borrow(), vec!["stage", "reconcile"]);
}
#[test]
fn health_precedes_promotion() {
    let f = Fake {
        events: RefCell::new(vec![]),
        healthy: true,
    };
    let c = Config::parse(include_str!("../gnx.toml")).unwrap();
    let r = app::execute("apply", &c, &f, &f, &f);
    assert_eq!(r.exit(), 0);
    assert_eq!(*f.events.borrow(), vec!["stage", "reconcile", "promote"]);
}
