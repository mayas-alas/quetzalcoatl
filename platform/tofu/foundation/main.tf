variable "node_name" {
  type = string

  validation {
    condition     = can(regex("^gnx-controller-[a-z0-9-]+$", var.node_name)) && length(var.node_name) <= 63
    error_message = "node_name must be the persisted controller hostname."
  }
}

locals {
  foundation = {
    garage = {
      vm_id       = 200
      hostname    = "gnx-garage"
      description = "GNX platform object storage"
      cores       = 2
      memory      = 2048
      disk        = 40
    }
    forgejo = {
      vm_id       = 201
      hostname    = "gnx-forgejo"
      description = "GNX platform source and OCI registry"
      cores       = 2
      memory      = 3072
      disk        = 40
    }
    runner = {
      vm_id       = 202
      hostname    = "gnx-runner"
      description = "GNX isolated Forgejo Actions runner"
      cores       = 2
      memory      = 3072
      disk        = 40
    }
  }
}

resource "proxmox_download_file" "debian_13" {
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

resource "proxmox_virtual_environment_container" "foundation" {
  for_each = local.foundation

  node_name     = var.node_name
  vm_id         = each.value.vm_id
  description   = each.value.description
  tags          = ["gnx", "platform", each.key]
  unprivileged  = true
  started       = true
  start_on_boot = true

  cpu {
    cores = each.value.cores
  }

  memory {
    dedicated = each.value.memory
    swap      = 512
  }

  features {
    nesting = true
    fuse    = true
    keyctl  = true
    mknod   = true
  }

  initialization {
    hostname = each.value.hostname

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
    size         = each.value.disk
  }

  operating_system {
    template_file_id = proxmox_download_file.debian_13.id
    type             = "debian"
  }

  device_passthrough {
    path = "/dev/net/tun"
    mode = "0666"
  }
}

output "foundation" {
  value = {
    for name, container in proxmox_virtual_environment_container.foundation :
    name => {
      vm_id    = container.vm_id
      hostname = local.foundation[name].hostname
    }
  }
}
