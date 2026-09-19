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
    fn stage(&self, _: &Config) -> Result<(), String> {
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
