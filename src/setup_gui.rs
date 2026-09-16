use eframe::egui;
use quetzalcoatl_gnx::{installer::{Stage, State}, windows::installer as engine, PRODUCT};
use std::time::{Duration, Instant};

pub fn show() -> Result<(), String> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_title(format!("{PRODUCT} · Instalación")).with_inner_size([720.0, 560.0]).with_min_inner_size([600.0, 480.0]),
        ..Default::default()
    };
    eframe::run_native(PRODUCT, options, Box::new(|cc| {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Ok(Box::new(Assistant::new()))
    })).map_err(|e| e.to_string())
}
struct Assistant {
    sid: String,
    state: Option<State>,
    error: Option<String>,
    last_poll: Instant,
    worker: Option<engine::Worker>,
    countdown: Option<Instant>,
    diagnostic: Option<String>,
}
impl Assistant {
    fn new() -> Self {
        let sid = engine::caller_sid();
        Self { sid: sid.clone().unwrap_or_default(), state: None, error: sid.err(), last_poll: Instant::now() - Duration::from_secs(2), worker: None, countdown: None, diagnostic: None }
    }
    fn action(&mut self, action: &str) {
        match engine::request_action(action, &self.sid) {
            Ok(worker) => { self.worker = Some(worker); self.error = None; }
            Err(e) => self.error = Some(e),
        }
    }
}
impl eframe::App for Assistant {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(200));
        if self.last_poll.elapsed() > Duration::from_secs(1) {
            match engine::load_state() { Ok(s) => self.state = s, Err(e) => self.error = Some(e) }
            self.last_poll = Instant::now();
        }
        if let Some(worker) = &self.worker {
            match worker.exit_code() {
                Ok(None) => (),
                Ok(Some(code)) => {
                    self.worker = None;
                    if code != 0 && code != 3010 { self.error = Some(format!("El motor terminó con código {code}. Revisa el diagnóstico; comprueba permisos y que ambos EXE estén juntos.")); }
                }
                Err(e) => { self.worker = None; self.error = Some(e); }
            }
        }
        let pending = self.worker.is_some();
        let state = self.state.clone();
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(16.0);
            ui.heading(PRODUCT);
            ui.label("Entorno Linux dedicado · Ubuntu 24.04 · Podman");
            ui.add_space(12.0); ui.separator(); ui.add_space(12.0);
            if let Some(s) = &state {
                ui.heading(s.stage.label());
                ui.add_space(8.0);
                ui.label(&s.detail);
                if s.stage.busy() { ui.horizontal(|ui| { ui.spinner(); ui.label("Operación en curso; no se estima un porcentaje."); }); }
                ui.add_space(16.0);
                for (stage, label) in [
                    (Stage::Preparing, "1. Preparar Windows y WSL"),
                    (Stage::Provisioning, "2. Preparar cuenta y servicio"),
                    (Stage::Downloading, "3. Descargar Ubuntu"),
                    (Stage::Configuring, "4. Configurar systemd y Podman"),
                    (Stage::Verifying, "5. Verificar el entorno"),
                ] {
                    if s.stage == stage { ui.strong(format!("→ {label}")); } else { ui.label(label); }
                }
                ui.add_space(12.0);
                if s.client_sid != self.sid { ui.colored_label(egui::Color32::YELLOW, "Esta instalación pertenece a otro usuario. Abre el asistente desde esa cuenta."); }
                else if s.stage == Stage::RebootRequired {
                    ui.label("Guarda tus documentos. El reinicio no forzará el cierre de aplicaciones.");
                    if let Some(deadline) = self.countdown {
                        let remaining = deadline.saturating_duration_since(Instant::now());
                        ui.strong(format!("Solicitar reinicio en {} segundos", remaining.as_secs()));
                        if ui.button("Cancelar reinicio / Más tarde").clicked() { self.countdown = None; }
                        else if remaining.is_zero() { self.countdown = None; self.action("--restart"); }
                    } else if ui.add_enabled(!pending, egui::Button::new("Reiniciar y continuar (30 s) …")).clicked() {
                        self.countdown = Some(Instant::now() + Duration::from_secs(30));
                    }
                    ui.label("Puedes cerrar esta ventana y reiniciar más tarde. El motor continuará al arrancar y esta ventana reaparecerá al iniciar sesión.");
                } else if s.stage == Stage::Failed {
                    if ui.add_enabled(!pending, egui::Button::new("Reintentar con permisos de administrador")).clicked() { self.action("--start"); }
                    ui.label("No se borrarán distribuciones ni cuentas existentes. Algunas instalaciones parciales requieren revisión administrativa.");
                } else if s.stage == Stage::Complete {
                    ui.colored_label(egui::Color32::LIGHT_GREEN, "✓ El entorno respondió a las verificaciones de salud.");
                } else {
                    ui.label("Puedes cerrar el asistente: el trabajo continúa en segundo plano.");
                    if ui.add_enabled(!pending, egui::Button::new("Reanudar si el motor se interrumpió")).clicked() { self.action("--start"); }
                }
            } else {
                ui.heading("Preparar este equipo");
                ui.label("Se habilitará WSL, se creará una cuenta Windows dedicada sin privilegios de administrador y se descargará Ubuntu 24.04. Requiere Internet y puede necesitar reiniciar.");
                ui.add_space(12.0);
                ui.label("No se modificarán tus otras distribuciones. Windows solicitará autorización para iniciar el motor de instalación.");
                ui.add_space(20.0);
                if ui.add_enabled(!pending && !self.sid.is_empty(), egui::Button::new("Instalar")).clicked() { self.action("--start"); }
                if pending { ui.spinner(); ui.label("Esperando al motor de instalación…"); }
            }
            if let Some(error) = &self.error { ui.add_space(10.0); ui.colored_label(egui::Color32::LIGHT_RED, error); }
            ui.add_space(16.0); ui.separator();
            ui.label("Diagnóstico del instalador:");
            ui.horizontal_wrapped(|ui| {
                ui.monospace(engine::diagnostic_path().display().to_string());
                if ui.button("Ver diagnóstico").clicked() {
                    use std::io::{Read, Seek, SeekFrom};
                    self.diagnostic = Some((|| -> Result<String, std::io::Error> {
                        let mut file = std::fs::File::open(engine::diagnostic_path())?;
                        let start = file.metadata()?.len().saturating_sub(65536);
                        file.seek(SeekFrom::Start(start))?;
                        let mut bytes = Vec::new(); file.take(65536).read_to_end(&mut bytes)?;
                        Ok(String::from_utf8_lossy(&bytes).into_owned())
                    })().unwrap_or_else(|e| format!("No se pudo leer el diagnóstico: {e}")));
                }
                if ui.button("Copiar ruta").clicked() { ui.ctx().copy_text(engine::diagnostic_path().display().to_string()); }
            });
            ui.small("Versión experimental. La instalación completa y la reanudación deben validarse con un reinicio real.");
        });
        if let Some(text) = &self.diagnostic {
            let mut open = true;
            egui::Window::new("Diagnóstico · últimas 64 KiB").open(&mut open).default_size([640.0, 360.0]).show(ctx, |ui| {
                egui::ScrollArea::both().show(ui, |ui| { ui.monospace(text); });
            });
            if !open { self.diagnostic = None; }
        }
    }
}
