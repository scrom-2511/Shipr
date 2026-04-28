use crate::{
    app_errors::AppError,
    infra::{detect::ProjectType, process::run_script},
};
use std::{
    process::{Command, Stdio},
    thread::sleep,
    time::Duration,
};

pub struct Firecracker {
    api_socket: String,
    unique_id: u32,
    client: reqwest::Client,
    base_id: u32,
    lang: ProjectType,
}

impl Firecracker {
    pub fn new(unique_id: u32, lang: ProjectType) -> Self {
        let api_socket = format!("/tmp/firecracker-{}.socket", unique_id);

        let path: &str = api_socket.as_ref();

        let client = reqwest::Client::builder()
            .unix_socket(path)
            .build()
            .unwrap();

        let base_id = unique_id * 4;

        Self {
            api_socket,
            unique_id,
            client,
            base_id,
            lang,
        }
    }

    pub fn get_base_id(&self) -> u32 {
        self.unique_id
    }

    fn init_vm(&mut self) -> Result<(), AppError> {
        let cmd_1 = format!(r#"sudo rm -f {}"#, self.api_socket);
        let cmd_2 = format!(
            r#"./firecracker --api-sock {} --enable-pci"#,
            self.api_socket
        );

        run_script(vec![&cmd_1])?;

        Command::new("bash")
            .arg("-c")
            .arg(cmd_2)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .current_dir("/home/scrom")
            .spawn()
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;

        let path: &str = self.api_socket.as_ref();

        self.client = reqwest::Client::builder()
            .unix_socket(path)
            .build()
            .unwrap();

        Ok(())
    }

    fn setup_network(&self) -> Result<(), AppError> {
        let tap_dev = format!("tap{}", self.unique_id);
        let tap_ip = format!("172.16.0.{}", self.base_id + 1);
        println!("tap_ip: {}", tap_ip);
        let mask_short = "/30";

        let cmd_1 = format!(r#"sudo ip link del {} 2> /dev/null || true"#, tap_dev);
        let cmd_2 = format!(r#"sudo ip tuntap add dev {} mode tap"#, tap_dev);
        let cmd_3 = format!(
            r#"sudo ip addr add {}{} dev {}"#,
            tap_ip, mask_short, tap_dev
        );
        let cmd_4 = format!(r#"sudo ip link set dev {} up"#, tap_dev);

        run_script(vec![&cmd_1, &cmd_2, &cmd_3, &cmd_4])?;

        Ok(())
    }

    async fn set_machine_config(&self) -> Result<(), AppError> {
        let data = serde_json::json!({
            "vcpu_count": 1,
            "mem_size_mib": 1024,
        });

        self.client
            .put("http://localhost/machine-config")
            .json(&data)
            .send()
            .await
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;

        Ok(())
    }

    async fn set_boot_source(&self) -> Result<(), AppError> {
        let kernel = "/home/scrom/vmlinux-6.1.155";
        let kernel_boot_args = "console=ttyS0 reboot=k panic=1";

        let data = serde_json::json!({
            "kernel_image_path": kernel,
            "boot_args": kernel_boot_args
        });

        self.client
            .put("http://localhost/boot-source")
            .json(&data)
            .send()
            .await
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;

        Ok(())
    }

    async fn set_rootfs(&self) -> Result<(), AppError> {
        let rootfs_path = format!("rootfs-nodejs.ext4");
        println!("rootfs_path: {}", rootfs_path);
        let copy_rootfs = format!(r#"cp {} rootfs-{}.ext4"#, rootfs_path, self.unique_id);

        run_script(vec![&copy_rootfs])?;

        let rootfs = format!("/home/scrom/rootfs-{}.ext4", self.unique_id);

        let data = serde_json::json!({
            "drive_id": "rootfs",
            "path_on_host": rootfs,
            "is_root_device": true,
            "is_read_only": false
        });

        self.client
            .put("http://localhost/drives/rootfs")
            .json(&data)
            .send()
            .await
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;

        Ok(())
    }

    async fn set_network_interface(&self) -> Result<(), AppError> {
        let tap_dev = format!("tap{}", self.unique_id);

        let vm_last_octet = self.base_id + 2;

        let fc_mac = format!(
            "06:00:{:02X}:{:02X}:{:02X}:{:02X}",
            172, 16, 0, vm_last_octet
        );

        let data = serde_json::json!({
            "iface_id": "net1",
            "guest_mac": fc_mac,
            "host_dev_name": tap_dev
        });

        self.client
            .put("http://localhost/network-interfaces/net1")
            .json(&data)
            .send()
            .await
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;

        Ok(())
    }

    async fn start_instance(&self) -> Result<(), AppError> {
        let data = serde_json::json!({
            "action_type": "InstanceStart"
        });

        self.client
            .put("http://localhost/actions")
            .json(&data)
            .send()
            .await
            .map_err(|e| AppError::StartingFirecrackerFailed(e.to_string()))?;

        sleep(Duration::from_secs(3));

        Ok(())
    }

    pub fn execute_command(&self, command: &str) -> Result<(), AppError> {
        let cmd = format!(
            r#"ssh -t -i ubuntu.id_rsa root@172.16.0.{} 'bash -i -c "{}"'"#,
            self.base_id + 2,
            command
        );

        run_script(vec![&cmd])?;

        Ok(())
    }

    async fn setup_ssh(&self) -> Result<(), AppError> {
        self.execute_command("ip route add default via 172.16.0.1 dev eth0")?;
        self.execute_command("echo 'nameserver 8.8.8.8' > /etc/resolv.conf")?;

        Ok(())
    }

    pub async fn destroy_vm(&self) -> Result<(), AppError> {
        self.execute_command("reboot")?;
        let cmd_1 = format!(
            r#"sudo ip link del tap{} 2> /dev/null || true"#,
            self.unique_id
        );

        let cmd_2 = format!(r#"rm rootfs-{}.ext4"#, self.unique_id);

        run_script(vec![&cmd_1, &cmd_2])?;

        Ok(())
    }

    async fn all_setup(&mut self) -> Result<(), AppError> {
        self.setup_network()?;
        println!("network setup done");

        self.set_machine_config().await?;
        println!("machine config set");

        self.set_boot_source().await?;
        println!("boot source set");

        self.set_rootfs().await?;
        println!("rootfs set");

        self.set_network_interface().await?;
        println!("network interface set");

        self.start_instance().await?;
        println!("instance started");

        Ok(())
    }

    pub async fn create_vm(&mut self) -> Result<(), AppError> {
        self.init_vm()?;
        sleep(Duration::from_secs(3));
        self.setup_network()?;
        self.set_machine_config().await?;
        self.set_boot_source().await?;
        self.set_rootfs().await?;
        self.set_network_interface().await?;
        self.start_instance().await?;
        self.setup_ssh().await?;

        println!("VM created successfully");

        Ok(())
    }
}
