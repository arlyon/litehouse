# terraform {
#   backend "s3" {
#     bucket = "litehouse-tf"
#     key    = "tfstate"
#     region = "eu-central-1"
#     endpoint = "https://u5f5.fra2.idrivee2-11.com"
#     skip_credentials_validation = true
#   }
# }

variable "hcloud_token" {
  description = "Hetzner Cloud API Token"
  type        = string
}

output "ignition" {
  value = data.ct_config.web.rendered
}

variable "ssh_public_key_file" {
  description = "Local path to your public key"
  type        = string
  default     = "~/.ssh/id_ed25519.pub"
}

variable "ssh_private_key_file" {
  description = "Local path to your private key"
  type        = string
  default     = "~/.ssh/id_ed25519"
}

variable "ssh_public_key_name" {
  description = "Name of your public key to identify at Hetzner Cloud portal"
  type        = string
  default     = "litehouse-ssh-key"
}

variable "hcloud_server_type" {
  description = "vServer type name, lookup via `hcloud server-type list`"
  type        = string
  default     = "cx32"
}

variable "hcloud_server_datacenter" {
  description = "Desired datacenter location name, lookup via `hcloud datacenter list`"
  type        = string
  default     = "nbg1-dc3"
}

variable "hcloud_server_name" {
  description = "Name of the server"
  type        = string
  default     = "litehouse-cockpit"
}

# Update version to the latest release of Butane
variable "tools_butane_version" {
  description = "See https://github.com/coreos/butane/releases for available versions"
  type        = string
  default     = "0.21.0"
}



####
# Infrastructure config
##

provider "hcloud" {
  token = var.hcloud_token
}

provider "ct" {}

resource "hcloud_ssh_key" "key" {
  name       = var.ssh_public_key_name
  public_key = file(var.ssh_public_key_file)
}

data "ct_config" "web" {
  content = templatefile("coreos/config.yml",
    {
      ssh_public_key = trimspace(file(var.ssh_public_key_file))
    }
  )
  strict       = true
  pretty_print = false
  files_dir    = "coreos"
}

resource "hcloud_server" "master" {
  name   = var.hcloud_server_name
  labels = { "os" = "coreos" }

  server_type = var.hcloud_server_type
  datacenter  = var.hcloud_server_datacenter

  image    = "fedora-39" # image is ignored, as we boot into rescue mode, but is a required field
  rescue   = "linux64"
  ssh_keys = [hcloud_ssh_key.key.id]

  connection {
    host        = self.ipv4_address
    timeout     = "5m"
    private_key = file(var.ssh_private_key_file)
    # Root is the available user in rescue mode
    user = "root"
  }

  # Wait for the server to be available
  provisioner "local-exec" {
    command = "until nc -zv ${self.ipv4_address} 22; do sleep 5; done"
  }

  # Copy config.yaml and replace $ssh_public_key variable
  provisioner "file" {
    content     = data.ct_config.web.rendered
    destination = "/root/config.ign"
  }

  # Copy coreos-installer binary, as initramfs has not sufficient space to compile it in rescue mode
  provisioner "file" {
    source      = "coreos/coreos-installer-x86"
    destination = "/usr/local/bin/coreos-installer"
  }

  # Install Butane in rescue mode
  provisioner "remote-exec" {
    inline = [
      "set -x",
      # Download and install Fedora CoreOS to /dev/sda
      "chmod +x /usr/local/bin/coreos-installer",
      "coreos-installer install /dev/sda -i /root/config.ign",
      # Exit rescue mode and boot into coreos
      "reboot"
    ]
  }

  # Wait for the server to be available
  provisioner "local-exec" {
    command = "until nc -zv ${self.ipv4_address} 22; do sleep 15; done"
  }

  # Configure CoreOS after installation
  provisioner "remote-exec" {
    connection {
      host        = self.ipv4_address
      timeout     = "1m"
      private_key = file(var.ssh_private_key_file)
      # This user is configured in config.yaml
      user = "core"
    }

    inline = [
      "sudo hostnamectl set-hostname ${self.name}"
      # Add additional commands if needed
    ]
  }
}
