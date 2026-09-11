pub fn render(c: &crate::config::Config, ip: &str, compute_name: &str, compute_ip: &str) -> String {
    let mut text=format!("{{\n admin off\n auto_https disable_redirects\n servers {{\n strict_sni_host on\n }}\n}}\nhttps://compute.gnx {{\n bind {ip}\n tls internal\n reverse_proxy https://{compute_ip}:8006 {{\n header_up Host {compute_name}\n transport http {{\n tls_trust_pool file /gnx-compute-ca.pem\n tls_server_name {compute_name}\n }}\n }}\n}}\n");
    text = text.replacen(" admin off\n", " admin off\n pki {\n ca local {\n name \"GNX Local Auth\"\n root_cn \"GNX Root\"\n intermediate_cn \"GNX Local Auth\"\n }\n }\n", 1);
    text = text.replacen(
        "https://compute.gnx {",
        "https://compute.gnx, https://proxmox.gnx {",
        1,
    );
    text.push_str(&format!("https://app.gnx {{\n bind {ip}\n tls internal\n root * /gnx-app\n header X-Content-Type-Options nosniff\n header Referrer-Policy no-referrer\n file_server\n}}\n"));
    for r in &c.routes {
        text.push_str(&format!(
            "https://{} {{\n bind {ip}\n tls internal\n reverse_proxy {}\n}}\n",
            r.hostname, r.upstream
        ));
    }
    text
}
