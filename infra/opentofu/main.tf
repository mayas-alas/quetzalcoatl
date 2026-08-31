locals {
  ubuntu_image_url    = "https://cloud-images.ubuntu.com/releases/noble/release-20260826/ubuntu-24.04-server-cloudimg-amd64-root.tar.xz"
  ubuntu_image_sha256 = "df1146e4f2bc372b193c966b709f1b5e22a5facb27721ad80c5bae254040c380"
}

resource "proxmox_download_file" "ubuntu_lxc" {
  count = var.lxc_enabled ? 1 : 0

  content_type       = "vztmpl"
  datastore_id       = "local"
  file_name          = "ubuntu-24.04-20260826-amd64-root.tar.xz"
  node_name          = var.node_name
  url                = local.ubuntu_image_url
  checksum           = local.ubuntu_image_sha256
  checksum_algorithm = "sha256"
  overwrite          = false
  upload_timeout     = 1800
}

resource "proxmox_virtual_environment_container" "cell" {
  count = var.lxc_enabled ? 1 : 0

  description   = "Quetzalcoatl Next managed cell"
  node_name     = var.node_name
  vm_id         = var.vm_id
  started       = true
  start_on_boot = true
  unprivileged  = false
  tags          = ["gnx", "managed"]

  cpu {
    cores = 2
  }

  memory {
    dedicated = 2048
    swap      = 512
  }

  features {
    nesting = true
    fuse    = true
    keyctl  = true
    mknod   = true
  }

  device_passthrough {
    path = "/dev/net/tun"
  }

  disk {
    datastore_id = "local"
    size         = 16
  }

  operating_system {
    template_file_id = proxmox_download_file.ubuntu_lxc[0].id
    type             = "ubuntu"
  }

  initialization {
    hostname   = "gnx-cell-01"
    entrypoint = "/opt/gnx/guest/bootstrap.sh"

    ip_config {
      ipv4 {
        address = "dhcp"
      }
    }

    user_account {
      password = var.guest_password
    }
  }

  network_interface {
    bridge = "vmbr0"
    name   = "eth0"
  }

  mount_point {
    path      = "/opt/gnx/guest"
    read_only = true
    volume    = "/opt/gnx/guest"
  }

  startup {
    order      = "10"
    up_delay   = "10"
    down_delay = "30"
  }

  wait_for_ip {
    ipv4 = true
  }
}
