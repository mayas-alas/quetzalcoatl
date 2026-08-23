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

variable "service_hostname_prefix" {
  type = string

  validation {
    condition     = can(regex("^gnx-svc-[a-z0-9][a-z0-9-]{0,31}$", var.service_hostname_prefix)) && length(var.service_hostname_prefix) <= 60
    error_message = "service_hostname_prefix must use the canonical GNX service prefix."
  }
}

variable "vm_id_start" {
  type = number

  validation {
    condition     = var.vm_id_start >= 300 && var.vm_id_start <= 7998 && floor(var.vm_id_start) == var.vm_id_start
    error_message = "vm_id_start must be an integer in the reserved workload range with room for count."
  }
}

variable "count" {
  type = number

  validation {
    condition     = var.count >= 1 && var.count <= 10 && floor(var.count) == var.count
    error_message = "count must be an integer between 1 and 10."
  }
}

resource "proxmox_virtual_environment_container" "service" {
  count         = var.count
  node_name     = var.node_name
  vm_id         = var.vm_id_start + count.index
  description   = "GNX managed service ${var.service_slug} instance ${count.index + 1}"
  tags          = ["gnx", "service", var.service_slug]
  unprivileged  = true
  started       = true
  start_on_boot = true

  cpu {
    cores = 1
  }

  memory {
    dedicated = 1024
    swap      = 512
  }

  features {
    nesting = true
    fuse    = true
    keyctl  = true
    mknod   = true
  }

  initialization {
    hostname = "${var.service_hostname_prefix}-${count.index + 1}"

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
    size         = 10
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

output "services" {
  value = [
    for i, c in proxmox_virtual_environment_container.service : {
      hostname = c.initialization[0].hostname
      slug     = var.service_slug
      vm_id    = c.vm_id
    }
  ]
}