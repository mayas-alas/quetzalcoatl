variable "node_name" {
  type = string

  validation {
    condition     = can(regex("^gnx-controller-[a-z0-9-]+$", var.node_name)) && length(var.node_name) <= 63
    error_message = "node_name must be the persisted controller hostname."
  }
}

variable "service_slug" {
  type = string

  validation {
    condition     = can(regex("^[a-z0-9][a-z0-9-]{0,31}$", var.service_slug))
    error_message = "service_slug must be a bounded canonical slug."
  }
}

variable "service_hostname" {
  type = string

  validation {
    condition     = can(regex("^gnx-[a-z0-9][a-z0-9-]{0,30}[a-z0-9]$", var.service_hostname)) && length(var.service_hostname) <= 63
    error_message = "service_hostname must use the canonical GNX service prefix and end in an alphanumeric."
  }
}

variable "service_vmid" {
  type = number

  validation {
    condition     = var.service_vmid >= 300 && var.service_vmid <= 7999 && floor(var.service_vmid) == var.service_vmid
    error_message = "service_vmid must be an integer in the reserved service range."
  }
}

resource "proxmox_virtual_environment_container" "service" {
  node_name     = var.node_name
  vm_id         = var.service_vmid
  description   = "GNX managed service ${var.service_slug}"
  tags          = ["gnx", "service", var.service_slug]
  unprivileged  = true
  started       = true
  start_on_boot = true

  cpu {
    cores = 2
  }

  memory {
    dedicated = 2048
    swap      = 1024
  }

  features {
    nesting = true
    fuse    = true
    keyctl  = true
    mknod   = true
  }

  initialization {
    hostname = var.service_hostname

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
    template_file_id = "local:vztmpl/debian-13-standard_13.6-1_amd64.tar.zst"
    type             = "debian"
  }

  device_passthrough {
    path = "/dev/net/tun"
    mode = "0666"
  }
}

output "service" {
  value = {
    hostname = var.service_hostname
    slug     = var.service_slug
    vm_id    = proxmox_virtual_environment_container.service.vm_id
  }
}
