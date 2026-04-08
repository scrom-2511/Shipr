use tokio::process::Command;

use crate::app_errors::AppError;

pub struct Firecracker;

impl Firecracker {
    pub fn new() -> Self {
        Self
    }

    async fn run_cmds(cmds: &Vec<&str>) -> Result<(), AppError> {
        if cmds.len() == 0 {
            return Ok(());
        }

        for cmd in cmds {
            let cmd_parts: Vec<&str> = cmd.split_whitespace().collect();

            print!("{:?}", cmd_parts);

            let initial_part = cmd_parts
                .get(0)
                .ok_or_else(|| AppError::CmdFailed("Empty command".into()))?;

            let output = Command::new(initial_part)
                .args(&cmd_parts[1..])
                .output()
                .await
                .map_err(|e| AppError::CmdFailed(e.to_string()))?;

            if !output.status.success() {
                return Err(AppError::CmdFailed(
                    String::from_utf8_lossy(&output.stderr).to_string(),
                ));
            }
        }

        Ok(())
    }

    pub async fn start() -> Result<(), AppError> {
        let API_SOCKET = "/tmp/firecracker.socket";

        let cleanup_cmd = format!("sudo rm -f {}", API_SOCKET);

        let firecracker_path = "/home/scrom/firecracker-main";

        Self::run_cmds(&vec![&cleanup_cmd]).await?;

        Command::new("sudo")
            .current_dir("/home/scrom")
            .arg(firecracker_path)
            .arg("--api-sock")
            .arg(API_SOCKET)
            .arg("--enable-pci")
            .spawn()
            .map_err(|e| AppError::CmdFailed(e.to_string()))?;

        Ok(())
    }

    async fn set_network_interface() -> Result<(), AppError> {
        let TAP_DEV = "tap0";
        let TAP_IP = "172.16.0.1";
        let MASK_SHORT = "/30";

        let cmd_1 = format!("sudo ip link del {} 2> /dev/null || true", TAP_DEV);
        let cmd_2 = format!("sudo ip tuntap add dev {} mode tap", TAP_DEV);
        let cmd_3 = format!("sudo ip addr add {}{} dev {}", TAP_IP, MASK_SHORT, TAP_DEV);
        let cmd_4 = format!("sudo ip link set dev {} up", TAP_DEV);

        Self::run_cmds(&vec![&cmd_1, &cmd_2, &cmd_3, &cmd_4]).await?;

        Ok(())
    }

    async fn enable_ip_forwarding() -> Result<(), AppError> {
        let cmd_1 = r#"sudo sh -c "echo 1 > /proc/sys/net/ipv4/ip_forward"#;
        let cmd_2 = r#"sudo iptables -P FORWARD ACCEPT"#;

        Self::run_cmds(&vec![&cmd_1, &cmd_2]).await?;

        Ok(())
    }

    async fn set_internet_access() -> Result<(), AppError> {
        let HOST_IFACE = r#"ip -j route list default |jq -r '.[0].dev"#;

        let cmd_1 = format!(
            "sudo iptables -t nat -D POSTROUTING -o {} -j MASQUERADE || true",
            HOST_IFACE
        );
        let cmd_2 = format!(
            "sudo iptables -t nat -A POSTROUTING -o {} -j MASQUERADE",
            HOST_IFACE
        );

        Self::run_cmds(&vec![&cmd_1, &cmd_2]).await?;

        Ok(())
    }

    async fn configure_firecracker() -> Result<(), AppError> {
        let API_SOCKET = "/tmp/firecracker.socket";
        let LOGFILE = "./firecracker.log";

        let cmd_logger = format!(
            r#"sudo curl -X PUT --unix-socket "{}" --data '{{ 
                "log_path": "{}", 
                "level": "Debug", 
                "show_level": true, 
                "show_log_origin": true 
            }}' http://localhost/logger"#,
            API_SOCKET, LOGFILE
        );

        let cmd_kernel = r#"KERNEL="./$(ls vmlinux* | tail -1)""#;
        let cmd_boot_args = r#"KERNEL_BOOT_ARGS="console=ttyS0 reboot=k panic=1""#;
        let cmd_arch = r#"ARCH=$(uname -m)"#;
        let cmd_arch_cond = r#"if [ "$ARCH" = "aarch64" ]; then KERNEL_BOOT_ARGS="keep_bootcon ${KERNEL_BOOT_ARGS}"; fi"#;

        let cmd_boot_source = format!(
            r#"sudo curl -X PUT --unix-socket "{}" --data '{{ 
                "kernel_image_path": '"$KERNEL"', 
                "boot_args": '"$KERNEL_BOOT_ARGS"' 
            }}' http://localhost/boot-source"#,
            API_SOCKET
        );

        let cmd_rootfs = r#"ROOTFS="./$(ls *.ext4 | tail -1)""#;

        let cmd_drive = format!(
            r#"sudo curl -X PUT --unix-socket "{}" --data '{{ 
                "drive_id": "rootfs", 
                "path_on_host": '"$ROOTFS"', 
                "is_root_device": true, 
                "is_read_only": false 
            }}' http://localhost/drives/rootfs"#,
            API_SOCKET
        );

        let cmd_mac = r#"FC_MAC="06:00:AC:10:00:02""#;

        let cmd_net = format!(
            r#"sudo curl -X PUT --unix-socket "{}" --data '{{ 
                "iface_id": "net1", 
                "guest_mac": '"$FC_MAC"', 
                "host_dev_name": "tap0" 
            }}' http://localhost/network-interfaces/net1"#,
            API_SOCKET
        );

        let cmd_sleep1 = "sleep 0.015";

        let cmd_start = format!(
            r#"sudo curl -X PUT --unix-socket "{}" --data '{{ 
                "action_type": "InstanceStart" 
            }}' http://localhost/actions"#,
            API_SOCKET
        );

        let cmd_sleep2 = "sleep 2";

        Self::run_cmds(&vec![
            &cmd_logger,
            cmd_kernel,
            cmd_boot_args,
            cmd_arch,
            cmd_arch_cond,
            &cmd_boot_source,
            cmd_rootfs,
            &cmd_drive,
            cmd_mac,
            &cmd_net,
            cmd_sleep1,
            &cmd_start,
            cmd_sleep2,
        ])
        .await?;

        Ok(())
    }

    async fn setup_guest_ssh() -> Result<(), AppError> {
        let cmd_key = r#"KEY_NAME=./$(ls *.id_rsa | tail -1)"#;

        let cmd_route =
            r#"ssh -i $KEY_NAME root@172.16.0.2 "ip route add default via 172.16.0.1 dev eth0""#;

        let cmd_dns =
            r#"ssh -i $KEY_NAME root@172.16.0.2 "echo 'nameserver 8.8.8.8' > /etc/resolv.conf""#;

        let cmd_ssh = r#"ssh -i $KEY_NAME root@172.16.0.2"#;

        Self::run_cmds(&vec![cmd_key, cmd_route, cmd_dns, cmd_ssh]).await?;

        Ok(())
    }

    pub async fn full_start() -> Result<(), AppError> {
        Self::start().await?;
        Self::set_network_interface().await?;
        Self::enable_ip_forwarding().await?;
        Self::set_internet_access().await?;
        Self::configure_firecracker().await?;

        Ok(())
    }
}
