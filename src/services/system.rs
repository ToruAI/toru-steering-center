use serde::{Deserialize, Serialize};
use sysinfo::{System, Disks, Networks};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuCore {
    pub name: String,
    pub usage: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo {
    pub name: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub used_space: u64,
    pub usage_percent: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub received: u64,
    pub transmitted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemResources {
    pub cpu_percent: f32,
    pub cpu_cores: Vec<CpuCore>,
    pub memory_percent: f32,
    pub memory_used: u64,
    pub memory_total: u64,
    pub swap_used: u64,
    pub swap_total: u64,
    pub uptime_seconds: u64,
    pub disks: Vec<DiskInfo>,
    pub network: Vec<NetworkInterface>,
    pub process_count: usize,
    pub system_name: Option<String>,
    pub kernel_version: Option<String>,
    pub os_version: Option<String>,
    pub host_name: Option<String>,
}

pub fn get_system_resources(sys: &mut System) -> SystemResources {
    sys.refresh_cpu_usage();
    sys.refresh_memory();
    sys.refresh_processes();
    
    // Get per-core CPU usage
    let cpus = sys.cpus();
    let cpu_cores: Vec<CpuCore> = cpus
        .iter()
        .enumerate()
        .map(|(i, cpu)| CpuCore {
            name: format!("Core {}", i),
            usage: cpu.cpu_usage(),
        })
        .collect();
    
    // Calculate average CPU usage
    let cpu_percent = if !cpus.is_empty() {
        cpus.iter().map(|c| c.cpu_usage()).sum::<f32>() / cpus.len() as f32
    } else {
        0.0
    };
    
    // Memory info
    let memory_total = sys.total_memory();
    let memory_used = sys.used_memory();
    let memory_percent = if memory_total > 0 {
        (memory_used as f32 / memory_total as f32) * 100.0
    } else {
        0.0
    };
    
    let swap_total = sys.total_swap();
    let swap_used = sys.used_swap();
    
    // Disk info
    let disks = Disks::new_with_refreshed_list();
    let disk_info: Vec<DiskInfo> = disks
        .iter()
        .map(|disk| {
            let total = disk.total_space();
            let available = disk.available_space();
            let used = total.saturating_sub(available);
            let usage_percent = if total > 0 {
                (used as f32 / total as f32) * 100.0
            } else {
                0.0
            };
            
            DiskInfo {
                name: disk.name().to_string_lossy().to_string(),
                mount_point: disk.mount_point().to_string_lossy().to_string(),
                total_space: total,
                available_space: available,
                used_space: used,
                usage_percent,
            }
        })
        .collect();
    
    // Network info
    let networks = Networks::new_with_refreshed_list();
    let network_info: Vec<NetworkInterface> = networks
        .iter()
        .map(|(name, data)| NetworkInterface {
            name: name.clone(),
            received: data.total_received(),
            transmitted: data.total_transmitted(),
        })
        .collect();
    
    let uptime_seconds = System::uptime();
    let process_count = sys.processes().len();
    
    SystemResources {
        cpu_percent,
        cpu_cores,
        memory_percent,
        memory_used,
        memory_total,
        swap_used,
        swap_total,
        uptime_seconds,
        disks: disk_info,
        network: network_info,
        process_count,
        system_name: System::name(),
        kernel_version: System::kernel_version(),
        os_version: System::os_version(),
        host_name: System::host_name(),
    }
}
