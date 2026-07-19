variable "node_name" {
  type = string

  validation {
    condition     = can(regex("^gnx-controller-[a-z0-9-]+$", var.node_name)) && length(var.node_name) <= 63
    error_message = "node_name must be the persisted controller hostname."
  }
}

variable "install_garage" {
  type = bool
}

variable "install_forgejo" {
  type = bool
}

locals {
  any_service = var.install_garage || var.install_forgejo
}

resource "proxmox_download_file" "debian_13" {
  count = local.any_service ? 1 : 0

  content_type       = "vztmpl"
  datastore_id       = "local"
  node_name          = var.node_name
  file_name          = "debian-13-standard_13.6-1_amd64.tar.zst"
  url                = "http://download.proxmox.com/images/system/debian-13-standard_13.6-1_amd64.tar.zst"
  checksum           = "4c0c27ca6ceab5ef0b84db57825a00f26157ef1854bafe97297813e1cbe8ecb8cc9c453cab6b3b0efe1ba193a50c47ece1e41d950e411b8730b835b71e9e754b"
  checksum_algorithm = "sha512"
  overwrite          = false
  upload_timeout     = 1800
}

resource "proxmox_virtual_environment_container" "garage" {
  count = var.install_garage ? 1 : 0

  node_name     = var.node_name
  vm_id         = 200
  description   = "Quetzalcoatl Garage service"
  tags          = ["quetzalcoatl", "garage"]
  unprivileged  = true
  started       = false
  start_on_boot = true

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

  initialization {
    hostname = "gnx-garage"

    dns {
      servers = ["172.30.10.1"]
    }

    ip_config {
      ipv4 {
        address = "dhcp"
      }
    }
  }

  network_interface {
    name   = "eth0"
    bridge = "vmbr0"
  }

  disk {
    datastore_id = "local"
    size         = 20
  }

  operating_system {
    template_file_id = proxmox_download_file.debian_13[0].id
    type             = "debian"
  }

  device_passthrough {
    path = "/dev/net/tun"
    mode = "0666"
  }

  device_passthrough {
    path = "/dev/fuse"
    mode = "0666"
  }
}

resource "proxmox_virtual_environment_container" "forgejo" {
  count = var.install_forgejo ? 1 : 0

  node_name     = var.node_name
  vm_id         = 201
  description   = "Quetzalcoatl Forgejo service"
  tags          = ["quetzalcoatl", "forgejo"]
  unprivileged  = true
  started       = false
  start_on_boot = true

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

  initialization {
    hostname = "gnx-forgejo"

    dns {
      servers = ["172.30.10.1"]
    }

    ip_config {
      ipv4 {
        address = "dhcp"
      }
    }
  }

  network_interface {
    name   = "eth0"
    bridge = "vmbr0"
  }

  disk {
    datastore_id = "local"
    size         = 20
  }

  operating_system {
    template_file_id = proxmox_download_file.debian_13[0].id
    type             = "debian"
  }

  device_passthrough {
    path = "/dev/net/tun"
    mode = "0666"
  }

  device_passthrough {
    path = "/dev/fuse"
    mode = "0666"
  }
}

output "garage_vmid" {
  value = try(proxmox_virtual_environment_container.garage[0].vm_id, null)
}

output "forgejo_vmid" {
  value = try(proxmox_virtual_environment_container.forgejo[0].vm_id, null)
}
