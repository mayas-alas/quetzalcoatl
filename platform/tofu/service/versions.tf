terraform {
  required_version = "= 1.12.4"

  required_providers {
    proxmox = {
      source  = "bpg/proxmox"
      version = "= 0.111.1"
    }
  }

  backend "s3" {}
}

provider "proxmox" {
  endpoint = "https://127.0.0.1:8006/"
  insecure = true
}
