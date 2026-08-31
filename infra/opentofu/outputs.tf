output "cell_vmid" {
  value = var.lxc_enabled ? proxmox_virtual_environment_container.cell[0].vm_id : null
}

output "cell_ipv4" {
  value = var.lxc_enabled ? proxmox_virtual_environment_container.cell[0].ipv4 : null
}
