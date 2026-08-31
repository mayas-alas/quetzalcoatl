variable "lxc_enabled" {
  description = "Create the first GNX LXC cell."
  type        = bool
  default     = true
}

variable "node_name" {
  description = "Dockur Proxmox node name."
  type        = string
  default     = "pve"
}

variable "vm_id" {
  description = "Stable VMID allocated to the first GNX cell."
  type        = number
  default     = 201

  validation {
    condition     = var.vm_id >= 100 && var.vm_id <= 999999999
    error_message = "vm_id must be in the Proxmox VMID range."
  }
}

variable "guest_password" {
  description = "Initial root password injected through TF_VAR_guest_password."
  type        = string
  sensitive   = true

  validation {
    condition     = length(var.guest_password) >= 32
    error_message = "guest_password must contain at least 32 characters."
  }
}
