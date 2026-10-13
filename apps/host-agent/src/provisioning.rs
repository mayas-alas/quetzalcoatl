use gnx_control_protocol::{NodeState, Operation, Progress, Request};
use uuid::Uuid;

/// Apply an authorized request and advance one durable checkpoint at a time.
/// Platform hooks are Windows-only so protocol tests never mutate the host.
pub fn execute(req: &Request, current: &Progress) -> Progress {
    // GetProgress is a read-only snapshot. It must not advance sequence,
    // replace the last request, clear diagnostics, or create a journal entry.
    if req.operation == Operation::GetProgress {
        // No WSL, profile mutex, verifier or runtime-version comparison here.
        // The existing status monitor reports live payload/health observations;
        // this command returns only the last durable checkpoint, even while an
        // upgrade/import holds Linux resources or the runtime is stopped.
        return current.clone();
    }
    if matches!(req.operation, Operation::ImportWorkload | Operation::DiagnoseWorkload) {
        let accepted = handle(req, current);
        if accepted.error_code.is_some() {
            return accepted;
        }
        return complete_long(req, &accepted, current);
    }
    let mut p = handle(req, current);
    if matches!(
        req.operation,
        Operation::Connect | Operation::Start | Operation::Stop | Operation::Status
    ) {
        #[cfg(windows)]
        let action_material = if req.operation == Operation::Connect {
            match consume_secrets(req, Operation::Connect) {
                Ok(material) => Some(material),
                Err(error) => {
                    p.phase = "github".into();
                    p.message = "GitHub Actions credential bundle was rejected.".into();
                    p.error_code = Some(classify_error(&error));
                    p.retryable = false;
                    return p;
                }
            }
        } else {
            None
        };
        #[cfg(not(windows))]
        let action_material: Option<crate::secret_store::SecretMaterial> = None;
        let action = match req.operation {
            Operation::Connect => "connect",
            Operation::Start => "start",
            Operation::Stop => "stop",
            Operation::Status => "status",
            _ => unreachable!(),
        };
        match crate::github::execute(
            action,
            &crate::state::state_dir(),
            req.github.as_ref(),
            p.github.as_ref(),
            action_material
                .as_ref()
                .and_then(|material| material.tailscale_auth_key.as_deref()),
        ) {
            Ok(status) => {
                p.github = Some(status.clone());
                p.phase = "github".into();
                p.message = status
                    .message
                    .clone()
                    .unwrap_or_else(|| "GitHub Actions status updated.".into());
                p.state = current.state.clone();
                p.percent = Some(100);
            }
            Err(error) => {
                p.github = Some(
                    p.github
                        .clone()
                        .unwrap_or_else(crate::github::initial_status),
                );
                p.phase = "github".into();
                p.message = "GitHub Actions operation did not complete.".into();
                p.state = current.state.clone();
                p.error_code = Some(classify_error(&error));
                p.retryable = true;
            }
        }
        return p;
    }
    if matches!(req.operation, Operation::Provision) && p.error_code.is_none() {
        #[cfg(windows)]
        let ca_result = account_then_ca(
            || crate::dedicated_account::ensure().map(|_| ()),
            || crate::ca::ensure(&crate::state::state_dir()),
        );
        #[cfg(not(windows))]
        let ca_result = crate::ca::ensure(&crate::state::state_dir());
        match ca_result {
            Ok(ca) => {
                p.private_ca_certificate = Some(ca.certificate_path.to_string_lossy().into());
                p.required_actions = required_actions(p.private_ca_certificate.as_deref());
            }
            Err(error) => {
                p.state = NodeState::Blocked;
                if let Some(account_error) = account_gate_source(&error) {
                    p.phase = "account".into();
                    p.message = "The managed Windows identity is not ready.".into();
                    p.error_code = Some(classify_error(account_error));
                } else {
                    p.phase = "ca".into();
                    p.message = "The node private CA could not be created or opened.".into();
                    p.error_code = Some(classify_error(&error));
                }
                p.retryable = true;
            }
        }
    }
    #[cfg(not(windows))]
    let _ = req;
    #[cfg(windows)]
    {
        if matches!(req.operation, Operation::Provision) && p.error_code.is_none() {
            if let Err(error) = provision_runtime(req) {
                p.state = NodeState::Blocked;
                p.phase = "windows".into();
                p.message = "Windows prerequisites need attention before WSL can continue.".into();
                p.error_code = Some(classify_error(&error));
                p.retryable = true;
                p.requires_restart = false;
            } else {
                p.state = NodeState::MeshPending;
                p.phase = "mesh".into();
                p.message = "Windows and WSL are ready; enter a reusable mesh key for protected Linux storage.".into();
                p.percent = Some(60);
            }
        }
        if matches!(req.operation, Operation::JoinMesh) && p.error_code.is_none() {
            // MSI updates can leave an already-imported distro with stale
            // payload files. Refresh and verify the sealed payload on every
            // JoinMesh, not only during Provision.
            if let Err(error) =
                crate::linux_node::ensure_distro().and_then(|_| join_mesh(req, &mut p))
            {
                let code = classify_error(&error);
                p.state = NodeState::Blocked;
                p.phase = "mesh".into();
                p.message = if code == "HOST_ACCESS_RECONCILE_FAILED" {
                    "Local Windows CA trust and loopback access could not be published.".into()
                } else {
                    "Mesh enrollment did not complete; no ready state was reported.".into()
                };
                p.error_code = Some(code);
                p.retryable = true;
            }
        }
        // A pipe client can disconnect or the service can restart after the
        // the reusable key was consumed but before NODE_READY was persisted. Retry
        // refreshes the installed payload and ingress policy without
        // credentials, then checks live readiness.
        if matches!(req.operation, Operation::Retry)
            && retry_can_reconcile(current)
        {
            let state_dir = crate::state::state_dir();
            let readiness = crate::linux_node::reconcile_ingress()
                .and_then(|_| crate::linux_node::verify(&state_dir))
                .and_then(|_| crate::linux_node::private_ip())
                .and_then(|ip| {
                    reconcile_host_access(&state_dir)?;
                    Ok(ip)
                });
            match readiness {
                Ok(ip) => mark_ready(&mut p, ip),
                Err(error) => {
                    let code = classify_error(&error);
                    p.state = NodeState::Blocked;
                    p.phase = "mesh".into();
                    p.message = if code == "HOST_ACCESS_RECONCILE_FAILED" {
                        "Local Windows CA trust and loopback access could not be published.".into()
                    } else {
                        "The enrolled runtime is not ready yet.".into()
                    };
                    p.error_code = Some(code);
                    p.retryable = true;
                }
            }
        }
        if matches!(req.operation, Operation::StopRuntime) && p.error_code.is_none() {
            if let Err(error) = crate::linux_node::stop_runtime() {
                p.state = current.state.clone();
                p.phase = current.phase.clone();
                p.message = "The managed GnX runtime could not be stopped safely.".into();
                p.error_code = Some(classify_error(&error));
                p.retryable = true;
            }
        }
        if matches!(req.operation, Operation::StartRuntime) && p.error_code.is_none() {
            let state_dir = crate::state::state_dir();
            let readiness = crate::linux_node::start_runtime()
                .and_then(|_| crate::linux_node::verify(&state_dir))
                .and_then(|_| crate::linux_node::private_ip())
                .and_then(|ip| {
                    reconcile_host_access(&state_dir)?;
                    Ok(ip)
                });
            match readiness {
                Ok(ip) => mark_ready(&mut p, ip),
                Err(error) => {
                    p.state = current.state.clone();
                    p.phase = "runtime-stopped".into();
                    p.message =
                        "The managed GnX runtime did not become healthy after start.".into();
                    p.error_code = Some(classify_error(&error));
                    p.retryable = true;
                }
            }
        }
        if matches!(req.operation, Operation::DiagnoseCompute) && p.error_code.is_none() {
            match crate::linux_node::diagnose_compute_file(&crate::state::state_dir()) {
                Ok(()) => {
                    p.state = current.state.clone();
                    p.phase = current.phase.clone();
                    p.message = "Compute diagnostic captured locally.".into();
                }
                Err(error) => {
                    p.state = current.state.clone();
                    p.phase = current.phase.clone();
                    p.message = "Compute diagnostic could not be captured.".into();
                    p.error_code = Some(classify_error(&error));
                }
            }
        }
        if matches!(
            req.operation,
            Operation::StartWorkload | Operation::StopWorkload
        ) && p.error_code.is_none()
        {
            let start = req.operation == Operation::StartWorkload;
            let workload = req.workload.as_deref().unwrap_or("rustfs");
            match crate::linux_node::control_workload(&crate::state::state_dir(), workload, start) {
                Ok(status) => {
                    p.state = current.state.clone();
                    p.phase = "workloads".into();
                    let observation_failed = matches!(status.state.as_str(), "ERROR" | "unknown")
                        || status.service == "unknown";
                    let attention = observation_failed
                        || status.service == "attention"
                        || status.state == "attention";
                    p.message = if observation_failed {
                        format!("The {workload} workload status could not be observed safely.")
                    } else if attention {
                        format!("The {workload} workload is running but requires attention; its adapter did not report healthy.")
                    } else if status.state == "running" {
                        format!("The {workload} workload is running and healthy.")
                    } else {
                        format!("The {workload} workload is stopped; persistent data is retained.")
                    };
                    if attention {
                        p.error_code = Some(if status.error_code.is_empty() {
                            "WORKLOAD_ATTENTION".into()
                        } else {
                            status.error_code.clone()
                        });
                        p.retryable = true;
                    } else {
                        p.error_code = None;
                        p.retryable = false;
                    }
                    p.percent = Some(100);
                }
                Err(error) => {
                    p.state = current.state.clone();
                    p.phase = "workloads".into();
                    p.message =
                        "The workload operation did not complete safely.".into();
                    p.error_code = Some(classify_error(&error));
                    p.retryable = true;
                }
            }
        }
        if req.operation == Operation::JoinWorkload && p.error_code.is_none() {
            let workload = req.workload.as_deref().unwrap_or_default();
            match join_workload(req, workload) {
                Ok(status) => {
                    p.state = current.state.clone();
                    p.phase = "workloads".into();
                    p.message = if status.service == "healthy" && status.state == "running" {
                        "The AI workload mesh is enrolled and healthy.".into()
                    } else {
                        "The AI workload mesh enrollment completed, but the workload requires attention; it is not reported healthy.".into()
                    };
                    if status.service != "healthy" || status.state != "running" {
                        p.error_code = Some(if status.error_code.is_empty() {
                            "WORKLOAD_ATTENTION".into()
                        } else {
                            status.error_code.clone()
                        });
                        p.retryable = true;
                    } else {
                        p.error_code = None;
                        p.retryable = false;
                    }
                    p.percent = Some(100);
                }
                Err(error) => {
                    p.state = current.state.clone();
                    p.phase = "workloads".into();
                    p.message = "The AI workload mesh enrollment did not complete safely.".into();
                    p.error_code = Some(classify_error(&error));
                    p.retryable = true;
                }
            }
        }
    }
    p
}

pub fn is_long_operation(operation: Operation) -> bool {
    matches!(operation, Operation::ImportWorkload | Operation::DiagnoseWorkload | Operation::Retry | Operation::StartWorkload | Operation::StopWorkload)
}

/// Create the durable accepted checkpoint without entering the long-running
/// admission/LXC path. The control pipe persists this before acknowledging.
pub fn begin_long(req: &Request, current: &Progress) -> Progress {
    let mut accepted = handle(req, current);
    // execute() already reconciles an existing enrollment without recapturing
    // initial setup credentials. Admission must not reject that supported
    // recovery path merely because handle() describes the secret-needed case.
    if req.operation == Operation::Retry && req.validate().is_ok()
        && current.request_id != Some(req.request_id)
        && allowed(Operation::Retry, current)
        && retry_can_reconcile(current)
    {
        accepted.error_code = None;
        accepted.retryable = false;
        accepted.phase = "runtime-reconcile".into();
        accepted.percent = Some(20);
        accepted.message = "Reconciling the existing enrolled runtime with installed source; workload health is observed separately.".into();
    }
    accepted
}

/// Finish a previously accepted long workload operation. The caller owns the
/// serialized mutation guard; read-only GetProgress does not enter it.
pub fn complete_long(req: &Request, accepted: &Progress, previous: &Progress) -> Progress {
    if matches!(req.operation, Operation::Retry | Operation::StartWorkload | Operation::StopWorkload) {
        let mut completed = execute(req, previous);
        completed.sequence = accepted.sequence + 1;
        return completed;
    }
    let mut p = accepted.clone();
    p.operation_id = Uuid::new_v4();
    p.sequence += 1;
    #[cfg(windows)]
    let result = match &req.operation {
        Operation::ImportWorkload => {
            let source = req.bundle_source.as_deref().ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "WORKLOAD_SOURCE_INVALID")
            });
            source.and_then(|source| {
                crate::workload_bundle::import(source, &crate::state::state_dir()).and_then(
                    |admission| {
                        crate::linux_node::deploy_bundle(
                            &admission.bundle_dir,
                            &admission.workload,
                            &admission.revision,
                        )
                        .map(|_| ())
                    },
                )
            })
        }
        Operation::DiagnoseWorkload => req
            .workload
            .as_deref()
            .ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, "WORKLOAD_NOT_ALLOWED")
            })
            .and_then(|workload| {
                crate::linux_node::diagnose_workload_file(
                    &crate::state::state_dir(),
                    workload,
                )
                .map(|_| ())
            }),
        _ => Err(std::io::Error::other("INVALID_LONG_OPERATION")),
    };
    #[cfg(not(windows))]
    let result: std::io::Result<()> = Err(std::io::Error::other(
        "WORKLOAD_OPERATION_UNAVAILABLE",
    ));
    match result {
        Ok(_) => {
            p.error_code = None;
            p.retryable = false;
            p.percent = Some(100);
            p.message = match &req.operation {
                Operation::ImportWorkload => "The workload bundle was admitted and deployed; runtime status remains authoritative.".into(),
                Operation::DiagnoseWorkload => "The AI workload diagnostic was captured locally.".into(),
                _ => p.message,
            };
        }
        Err(error) => {
            p.state = accepted.state.clone();
            p.error_code = Some(classify_error(&error));
            p.retryable = true;
            p.message = match &req.operation {
                Operation::ImportWorkload => "The workload bundle was not admitted or deployed safely.".into(),
                Operation::DiagnoseWorkload => "The AI workload diagnostic could not be captured safely.".into(),
                _ => "The workload operation did not complete safely.".into(),
            };
        }
    }
    p
}

fn required_actions(certificate: Option<&str>) -> Vec<gnx_control_protocol::RequiredAction> {
    vec![
        gnx_control_protocol::RequiredAction {
            kind: gnx_control_protocol::ActionKind::ConfigureSplitDns,
            target: "external tailnet clients: gnx".into(),
            value: None,
        },
        gnx_control_protocol::RequiredAction {
            kind: gnx_control_protocol::ActionKind::TrustPrivateCa,
            target: "external tailnet clients: app.gnx,compute.gnx".into(),
            value: certificate.map(str::to_owned),
        },
    ]
}

#[derive(Debug)]
struct AccountGateError(std::io::Error);

impl std::fmt::Display for AccountGateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(formatter)
    }
}

impl std::error::Error for AccountGateError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.0)
    }
}

fn account_gate_source(error: &std::io::Error) -> Option<&std::io::Error> {
    error
        .get_ref()
        .and_then(|source| source.downcast_ref::<AccountGateError>())
        .map(|source| &source.0)
}

/// Keep the account/profile/credential gate ahead of CA ACL application. The
/// Windows CA files grant read access to the fixed account, so account setup
/// must complete before CA creation can succeed. Keeping the two operations in
/// one helper also makes retries idempotent and keeps onboarding secrets out of
/// this prerequisite path.
fn account_then_ca<Account, Ca>(account: Account, ca: Ca) -> std::io::Result<crate::ca::CaInfo>
where
    Account: FnOnce() -> std::io::Result<()>,
    Ca: FnOnce() -> std::io::Result<crate::ca::CaInfo>,
{
    account().map_err(|error| std::io::Error::new(error.kind(), AccountGateError(error)))?;
    ca()
}

#[cfg(windows)]
fn consume_secrets(
    req: &Request,
    operation: Operation,
) -> std::io::Result<crate::secret_store::SecretMaterial> {
    let mut material = if let Some(path) = &req.secret_file {
        crate::secret_store::consume(std::path::Path::new(path))?
    } else {
        crate::secret_store::SecretMaterial {
            tailscale_auth_key: req.tailscale_auth_key.clone(),
            proxmox_password: req.proxmox_password.clone(),
        }
    };
    if operation == Operation::Provision && material.proxmox_password.is_none() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "PROXMOX_CREDENTIAL_MISSING",
        ));
    }
    if matches!(
        operation,
        Operation::JoinMesh | Operation::JoinWorkload | Operation::Connect
    ) && material.tailscale_auth_key.is_none()
    {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            if operation == Operation::Connect {
                "ACTIONS_TAILSCALE_CREDENTIAL_MISSING"
            } else {
                "MESH_CREDENTIAL_MISSING"
            },
        ));
    }
    Ok(std::mem::take(&mut material))
}

#[cfg(windows)]
fn join_workload(
    req: &Request,
    workload: &str,
) -> std::io::Result<crate::linux_node::WorkloadStatus> {
    if workload != "ai" {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "WORKLOAD_NOT_ALLOWED",
        ));
    }
    let mut material = consume_secrets(req, Operation::JoinWorkload)?;
    let mut key = material.tailscale_auth_key.take().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "MESH_CREDENTIAL_MISSING")
    })?;
    let path = crate::secret_store::staging_dir(&crate::state::state_dir())
        .join(format!("ai-mesh-{}.env", req.request_id));
    let result = (|| {
        crate::secret_store::stage_runtime_file(&path, "TS_AUTHKEY", &key)?;
        crate::linux_node::enroll_workload(&path)
    })();
    crate::secret_store::clear_string(&mut key);
    let _ = std::fs::remove_file(&path);
    result
}

#[cfg(windows)]
fn join_mesh(req: &Request, p: &mut Progress) -> std::io::Result<()> {
    let material = consume_secrets(req, Operation::JoinMesh)
        .map_err(|error| stage_error("MESH_SECRET_CONSUME_FAILED", error))?;
    let mut material = material;
    let mut key = material.tailscale_auth_key.take().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "MESH_CREDENTIAL_MISSING")
    })?;
    let staging = crate::secret_store::staging_dir(&crate::state::state_dir());
    let path = staging.join(format!("mesh-{}.env", req.request_id));
    // The runtime contract has independent environment files so each value
    // is consumed by its owning Quadlet and never appears in argv.
    let compute_path = staging.join(format!("compute-{}.env", req.request_id));
    let result = (|| {
        crate::secret_store::stage_runtime_file(&path, "TS_AUTHKEY", &key)
            .map_err(|error| stage_error("MESH_SECRET_STAGE_FAILED", error))?;
        if let Some(password) = material.proxmox_password.as_deref() {
            crate::secret_store::stage_runtime_file(&compute_path, "PROXMOX_PASSWORD", password)
                .map_err(|error| stage_error("COMPUTE_SECRET_STAGE_FAILED", error))?;
        }
        crate::linux_node::install_with_secrets(
            &crate::state::state_dir(),
            Some(&path),
            material
                .proxmox_password
                .as_ref()
                .map(|_| compute_path.as_path()),
        )
        .map_err(|error| stage_error("MESH_RUNTIME_INSTALL_FAILED", error))
    })();
    crate::secret_store::clear_string(&mut key);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&compute_path);
    result?;
    crate::linux_node::verify(&crate::state::state_dir())
        .map_err(|error| stage_error("MESH_RUNTIME_VERIFY_FAILED", error))?;
    let state_dir = crate::state::state_dir();
    let ip = crate::linux_node::private_ip()
        .map_err(|error| stage_error("MESH_PRIVATE_IP_FAILED", error))?;
    reconcile_host_access(&state_dir)?;
    mark_ready(p, ip);
    Ok(())
}

#[cfg(windows)]
fn reconcile_host_access(state_dir: &std::path::Path) -> std::io::Result<()> {
    crate::host_access::reconcile(&state_dir.join("ca/private-ca.cert.pem"))
        .map_err(|error| stage_error("HOST_ACCESS_RECONCILE_FAILED", error))
}

fn mark_ready(p: &mut Progress, ip: String) {
    p.state = NodeState::NodeReady;
    p.phase = "ready".into();
    p.message = "Node is ready locally; this Windows host trusts the CA and uses loopback access automatically. Complete the displayed actions only for external tailnet clients.".into();
    p.error_code = None;
    p.retryable = false;
    p.node_private_ip = Some(ip.clone());
    p.pihole_private_ip = Some(ip);
    p.tailscale_ip = p.node_private_ip.clone();
    p.pihole_ip = p.pihole_private_ip.clone();
    p.percent = Some(100);
}

fn stage_error(stage: &str, error: std::io::Error) -> std::io::Error {
    let text = error.to_string();
    let code = text.split_once(':').map(|(code, _)| code).unwrap_or(&text);
    let classified = !code.is_empty()
        && code.len() <= 80
        && code.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        });
    if classified && code != "ACCESS_DENIED" {
        error
    } else {
        std::io::Error::new(error.kind(), format!("{stage}: {error}"))
    }
}

fn classify_error(error: &std::io::Error) -> String {
    let text = error.to_string();
    let code = text.split_once(':').map(|(code, _)| code).unwrap_or(&text);
    if !code.is_empty()
        && code.len() <= 80
        && code.chars().all(|character| {
            character.is_ascii_uppercase() || character.is_ascii_digit() || character == '_'
        })
    {
        return code.into();
    }
    match error.kind() {
        std::io::ErrorKind::NotFound => "PAYLOAD_MISSING".into(),
        std::io::ErrorKind::PermissionDenied => "ACCESS_DENIED".into(),
        _ => "PROVISIONING_FAILED".into(),
    }
}

#[cfg(windows)]
fn provision_runtime(req: &Request) -> std::io::Result<()> {
    let material = consume_secrets(req, Operation::Provision)?;
    // This gate must precede every install.run invocation.  A missing or
    // unverified sealed image is an actionable release/installation blocker;
    // never silently use another registered distro or an arbitrary payload.
    crate::linux_node::ensure_distro()?;
    // The install path owns dependency installation before its strict KVM
    // ioctl gate. A pristine rootfs can contain the KVM modules without kmod,
    // so running the non-mutating check first would deadlock provisioning.
    let staging = crate::secret_store::staging_dir(&crate::state::state_dir());
    let mesh_path = staging.join(format!("mesh-{}.env", req.request_id));
    let compute_path = staging.join(format!("compute-{}.env", req.request_id));
    let result = (|| {
        if let Some(key) = material.tailscale_auth_key.as_deref() {
            crate::secret_store::stage_runtime_file(&mesh_path, "TS_AUTHKEY", key)?;
        }
        let password = material.proxmox_password.as_deref().ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "PROXMOX_CREDENTIAL_MISSING",
            )
        })?;
        crate::secret_store::stage_runtime_file(&compute_path, "PROXMOX_PASSWORD", password)?;
        crate::linux_node::install_with_secrets(
            &crate::state::state_dir(),
            material
                .tailscale_auth_key
                .as_ref()
                .map(|_| mesh_path.as_path()),
            Some(&compute_path),
        )
    })();
    let _ = std::fs::remove_file(&mesh_path);
    let _ = std::fs::remove_file(&compute_path);
    result.map(|_| ())
}

pub fn handle(req: &Request, current: &Progress) -> Progress {
    let mut p = current.clone();
    p.operation_id = Uuid::new_v4();
    p.sequence += 1;
    p.error_code = None;
    p.retryable = false;
    p.requires_restart = false;
    if req.operation != Operation::GetProgress && current.request_id == Some(req.request_id) {
        p.state = NodeState::RecoveryRequired;
        p.phase = "replay".into();
        p.message = "The request was already applied; submit a new request to continue.".into();
        p.error_code = Some("REPLAYED_REQUEST".into());
        p.retryable = false;
        return p;
    }
    p.request_id = Some(req.request_id);
    if req.validate().is_err() {
        p.state = NodeState::Failed;
        p.phase = "protocol".into();
        p.message = "The request was rejected by the control protocol.".into();
        p.error_code = Some("INVALID_REQUEST".into());
        return p;
    }
    if !allowed(req.operation.clone(), current) {
        // An invalid request must not destroy the last known-safe checkpoint.
        // Keep the real state/phase so a follow-up can choose the right
        // operation instead of recovering from a fabricated checkpoint.
        p.state = current.state.clone();
        p.phase = current.phase.clone();
        p.message = format!(
            "{:?} is not valid at the current checkpoint ({:?}); submit a supported operation.",
            req.operation, current.state,
        );
        p.error_code = Some("INVALID_TRANSITION".into());
        p.retryable = false;
        return p;
    }
    match req.operation {
        Operation::GetProgress => p,
        Operation::Provision => {
            p.state = NodeState::WindowsReady;
            p.phase = "windows".into();
            p.message = "Windows prerequisites are ready; continuing with WSL.".into();
            p.percent = Some(20);
            p
        }
        Operation::JoinMesh => {
            p.state = NodeState::MeshPending;
            p.phase = "mesh".into();
            p.message =
                "Mesh enrollment accepted; the reusable key is stored in protected Linux state.".into();
            p.percent = Some(80);
            p
        }
        Operation::Retry => {
            // Retry carries no secret by design. Never report a scheduled
            // operation when the next runtime step needs a reusable key.
            let (code, message) = match (&current.state, current.phase.as_str()) {
                (NodeState::MeshPending, _) | (NodeState::Blocked, "mesh") => (
                    "MESH_CREDENTIAL_MISSING",
                    "Retry needs a reusable mesh key; submit JoinMesh to store it safely.",
                ),
                (NodeState::Blocked, _) | (NodeState::RecoveryRequired, _) => (
                    "PROXMOX_CREDENTIAL_MISSING",
                    "Retry needs the one-shot provisioning credential; submit Provision.",
                ),
                _ => (
                    "RETRY_UNSUPPORTED",
                    "Retry cannot resume work at this checkpoint; submit a supported operation.",
                ),
            };
            p.state = current.state.clone();
            p.phase = current.phase.clone();
            p.message = message.into();
            p.error_code = Some(code.into());
            p.retryable = false;
            p
        }
        Operation::Cancel => {
            p.state = NodeState::RecoveryRequired;
            p.phase = "cancelled".into();
            p.message = "Operation cancelled at a safe checkpoint.".into();
            p
        }
        Operation::StopRuntime => {
            p.state = NodeState::RuntimeStopped;
            p.phase = "runtime-stopped".into();
            p.message =
                "The managed GnX runtime is stopped; its WSL memory has been released.".into();
            p.percent = Some(100);
            p
        }
        Operation::StartRuntime => {
            p.state = NodeState::RuntimeReady;
            p.phase = "runtime-starting".into();
            p.message = "Starting the managed GnX runtime.".into();
            p
        }
        Operation::StartWorkload => {
            p.state = current.state.clone();
            p.phase = "workloads".into();
            p.message = "Starting the managed private-services LXC.".into();
            p
        }
        Operation::StopWorkload => {
            p.state = current.state.clone();
            p.phase = "workloads".into();
            p.message =
                "Stopping the managed private-services LXC cleanly, including Pocket ID.".into();
            p
        }
        Operation::JoinWorkload => {
            p.state = current.state.clone();
            p.phase = "workloads".into();
            p.message = "Storing the reusable AI workload mesh key in protected Linux state.".into();
            p
        }
        Operation::ImportWorkload => {
            p.state = current.state.clone();
            p.phase = "workloads-import".into();
            p.message = "Admitting the workload bundle with the installed policy.".into();
            p
        }
        Operation::DiagnoseWorkload => {
            p.state = current.state.clone();
            // Diagnostics are observational and must not strand the durable
            // lifecycle checkpoint in a workload-specific phase.
            p.phase = current.phase.clone();
            p.message = "Capturing a bounded local AI workload diagnostic.".into();
            p
        }
        Operation::DiagnoseCompute => {
            p.state = current.state.clone();
            p.phase = current.phase.clone();
            p.message = "Capturing a bounded local compute diagnostic.".into();
            p
        }
        Operation::Connect | Operation::Start | Operation::Stop | Operation::Status => {
            p.state = current.state.clone();
            p.phase = "github".into();
            p.message = "Updating the native GitHub Actions integration.".into();
            p
        }
    }
}

fn allowed(operation: Operation, current: &Progress) -> bool {
    let state = &current.state;
    let has_enrolled_recovery = enrolled_recovery(current);
    match operation {
        Operation::GetProgress | Operation::Cancel => true,
        Operation::Provision => matches!(
            state,
            NodeState::New
                | NodeState::Blocked
                | NodeState::Failed
                | NodeState::RecoveryRequired
                | NodeState::RebootRequired
                | NodeState::NodeReady
        ),
        Operation::JoinMesh => matches!(
            state,
            NodeState::RuntimeReady | NodeState::MeshPending | NodeState::Blocked
        ),
        Operation::Retry => matches!(
            state,
            NodeState::Blocked
                | NodeState::Failed
                | NodeState::RecoveryRequired
                | NodeState::MeshPending
                | NodeState::NodeReady
        ),
        Operation::StopRuntime => matches!(state, NodeState::NodeReady) || has_enrolled_recovery,
        Operation::StartRuntime => matches!(state, NodeState::RuntimeStopped),
        Operation::StartWorkload | Operation::StopWorkload | Operation::JoinWorkload => {
            matches!(state, NodeState::NodeReady) || has_enrolled_recovery
        }
        Operation::ImportWorkload => matches!(state, NodeState::NodeReady) || has_enrolled_recovery,
        Operation::DiagnoseWorkload => !matches!(state, NodeState::New),
        Operation::DiagnoseCompute => !matches!(state, NodeState::New),
        Operation::Connect | Operation::Start | Operation::Stop | Operation::Status => true,
    }
}

/// A durable node address is the checkpoint's evidence that initial mesh
/// enrollment already happened. It permits recovery of a stranded lifecycle
/// operation without treating an un-enrolled blocked checkpoint as credentialless.
fn enrolled_recovery(current: &Progress) -> bool {
    current.node_private_ip.is_some()
        && matches!(
            &current.state,
            NodeState::Blocked | NodeState::Failed | NodeState::RecoveryRequired
        )
}

fn retry_can_reconcile(current: &Progress) -> bool {
    matches!(&current.state, NodeState::NodeReady | NodeState::MeshPending)
        || enrolled_recovery(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use gnx_control_protocol::PROTOCOL_VERSION;

    fn request(operation: Operation) -> Request {
        let key = (operation == Operation::JoinMesh).then(|| "tskey-auth-test".to_string());
        Request {
            version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            operation,
            tailscale_auth_key: key,
            proxmox_password: None,
            secret_file: None,
            github: None,
            workload: None,
            bundle_source: None,
        }
    }

    #[test]
    fn rejects_invalid_transition_without_mutating_checkpoint() {
        let mut current = crate::state::initial();
        current.state = NodeState::MeshPending;
        current.phase = "mesh".into();
        current.percent = Some(60);
        let next = handle(&request(Operation::Provision), &current);
        assert_eq!(next.state, NodeState::MeshPending);
        assert_eq!(next.phase, "mesh");
        assert_eq!(next.percent, Some(60));
        assert_eq!(next.error_code.as_deref(), Some("INVALID_TRANSITION"));
    }

    #[test]
    fn retry_requests_mesh_credential_instead_of_claiming_scheduled_work() {
        let mut current = crate::state::initial();
        current.state = NodeState::MeshPending;
        current.phase = "mesh".into();
        let next = handle(&request(Operation::Retry), &current);
        assert_eq!(next.state, NodeState::MeshPending);
        assert_eq!(next.error_code.as_deref(), Some("MESH_CREDENTIAL_MISSING"));
        assert!(!next.retryable);
        assert!(!next.message.contains("scheduled"));
    }

    #[test]
    fn retry_ambiguous_recovery_requests_provision_credential() {
        let mut current = crate::state::initial();
        current.state = NodeState::RecoveryRequired;
        current.phase = "checkpoint".into();
        let next = handle(&request(Operation::Retry), &current);
        assert_eq!(next.state, NodeState::RecoveryRequired);
        assert_eq!(next.phase, "checkpoint");
        assert_eq!(
            next.error_code.as_deref(),
            Some("PROXMOX_CREDENTIAL_MISSING")
        );
        assert!(!next.message.contains("JoinMesh"));
    }

    #[test]
    fn provision_starts_from_new() {
        let next = handle(&request(Operation::Provision), &crate::state::initial());
        assert_eq!(next.state, NodeState::WindowsReady);
        assert_eq!(next.sequence, 1);
    }

    #[test]
    fn provision_can_rotate_password_without_destroying_ready_enrollment() {
        let mut current = crate::state::initial();
        current.state = NodeState::NodeReady;
        current.phase = "ready".into();
        current.node_private_ip = Some("100.64.0.12".into());
        let next = handle(&request(Operation::Provision), &current);
        assert_eq!(next.state, NodeState::WindowsReady);
        assert_eq!(next.node_private_ip, current.node_private_ip);
        assert_ne!(next.error_code.as_deref(), Some("INVALID_TRANSITION"));
    }

    #[test]
    fn async_retry_admits_existing_ready_enrollment_without_faking_completion() {
        let mut current = crate::state::initial();
        current.state = NodeState::NodeReady;
        current.node_private_ip = Some("100.64.0.12".into());
        let accepted = begin_long(&request(Operation::Retry), &current);
        assert!(accepted.error_code.is_none());
        assert_eq!(accepted.phase, "runtime-reconcile");
        assert_eq!(accepted.percent, Some(20));
        assert_eq!(accepted.sequence, current.sequence + 1);
    }

    #[test]
    fn stranded_enrolled_workload_checkpoint_recovers_without_provisioning_credential() {
        let mut current = crate::state::initial();
        current.sequence = 41;
        current.state = NodeState::Blocked;
        current.phase = "workloads-diagnose".into();
        current.node_private_ip = Some("100.64.0.12".into());
        current.tailscale_ip = current.node_private_ip.clone();
        current.error_code = Some("MESH_STATUS_FAILED".into());
        current.retryable = true;

        let accepted = begin_long(&request(Operation::Retry), &current);
        assert_eq!(accepted.error_code, None);
        assert_eq!(accepted.phase, "runtime-reconcile");
        assert_eq!(accepted.sequence, current.sequence + 1);
        assert_eq!(accepted.node_private_ip, current.node_private_ip);

        let mut un_enrolled = current;
        un_enrolled.node_private_ip = None;
        let rejected = begin_long(&request(Operation::Retry), &un_enrolled);
        assert_eq!(
            rejected.error_code.as_deref(),
            Some("PROXMOX_CREDENTIAL_MISSING")
        );
    }

    #[test]
    fn workload_diagnostic_preserves_the_observed_lifecycle_phase() {
        let mut current = crate::state::initial();
        current.state = NodeState::Blocked;
        current.phase = "mesh".into();
        current.node_private_ip = Some("100.64.0.12".into());
        let mut diagnostic = request(Operation::DiagnoseWorkload);
        diagnostic.workload = Some("ai".into());

        let accepted = begin_long(&diagnostic, &current);
        assert_eq!(accepted.phase, current.phase);
        assert_eq!(accepted.state, current.state);
        assert_eq!(accepted.sequence, current.sequence + 1);
    }

    #[test]
    fn enrolled_attention_allows_protected_workload_recovery_not_fresh_bootstrap() {
        let mut current = crate::state::initial();
        current.state = NodeState::Blocked;
        assert!(!allowed(Operation::ImportWorkload, &current));
        assert!(!allowed(Operation::JoinWorkload, &current));
        current.node_private_ip = Some("100.64.0.12".into());
        assert!(allowed(Operation::ImportWorkload, &current));
        assert!(allowed(Operation::JoinWorkload, &current));
        assert!(allowed(Operation::StopRuntime, &current));
        assert_eq!(current.state, NodeState::Blocked);
    }

    #[test]
    fn get_progress_is_read_only() {
        let mut current = crate::state::initial();
        current.sequence = 7;
        current.error_code = Some("ACCOUNT_PROFILE_INVALID".into());
        current.retryable = true;
        let request = Request {
            version: PROTOCOL_VERSION,
            request_id: Uuid::new_v4(),
            operation: Operation::GetProgress,
            tailscale_auth_key: None,
            proxmox_password: None,
            secret_file: None,
            github: None,
            workload: None,
            bundle_source: None,
        };
        assert_eq!(execute(&request, &current), current);
    }

    #[test]
    fn provision_account_gate_precedes_ca_without_consuming_secrets() {
        use std::cell::RefCell;

        let events = RefCell::new(Vec::new());
        let result = account_then_ca(
            || {
                events.borrow_mut().push("account");
                Ok(())
            },
            || {
                events.borrow_mut().push("ca");
                Ok(crate::ca::CaInfo {
                    certificate_path: std::path::PathBuf::from("ca.cert.pem"),
                })
            },
        );
        assert!(result.is_ok());
        assert_eq!(*events.borrow(), ["account", "ca"]);
    }

    #[test]
    fn classifies_bare_runtime_codes_without_erasing_diagnostics() {
        let error = std::io::Error::other("KVM_UNAVAILABLE");
        assert_eq!(classify_error(&error), "KVM_UNAVAILABLE");
        let wrapped = std::io::Error::other("DISTRO_IMPORT_FAILED: details");
        assert_eq!(classify_error(&wrapped), "DISTRO_IMPORT_FAILED");
    }

    #[test]
    fn required_actions_are_external_tailnet_only() {
        let actions = required_actions(Some("ca/private-ca.cert.pem"));
        assert!(actions
            .iter()
            .all(|action| action.target.contains("external tailnet clients")));
    }

    #[test]
    fn account_gate_preserves_profile_error_and_phase_source() {
        let result = account_then_ca(
            || {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "ACCOUNT_PROFILE_ACCESS_INVALID: denied",
                ))
            },
            || panic!("CA must not run after an account failure"),
        );
        let error = result.expect_err("account failure should stop the gate");
        let source = account_gate_source(&error).expect("account source should be retained");
        assert_eq!(classify_error(source), "ACCOUNT_PROFILE_ACCESS_INVALID");
    }

    #[test]
    fn rejects_replayed_request_id() {
        let current = crate::state::initial();
        let first = request(Operation::Provision);
        let mut checkpoint = handle(&first, &current);
        let replay = handle(&first, &checkpoint);
        assert_eq!(replay.error_code.as_deref(), Some("REPLAYED_REQUEST"));
        assert_eq!(replay.state, NodeState::RecoveryRequired);
        checkpoint.request_id = Some(first.request_id);
        assert_eq!(
            handle(&first, &checkpoint).error_code.as_deref(),
            Some("REPLAYED_REQUEST")
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn provision_reports_ca_and_operator_actions() {
        let root = std::env::temp_dir().join(format!("gnx-progress-{}", Uuid::new_v4()));
        std::env::set_var("GNX_STATE_DIR", &root);
        let next = execute(&request(Operation::Provision), &crate::state::initial());
        assert!(next.private_ca_certificate.is_some());
        assert!(next
            .required_actions
            .iter()
            .any(|action| action.kind == gnx_control_protocol::ActionKind::ConfigureSplitDns));
        let _ = std::fs::remove_dir_all(root);
        std::env::remove_var("GNX_STATE_DIR");
    }
}
