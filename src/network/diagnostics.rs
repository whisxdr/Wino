//! Network Center: adapter state and native diagnostics.
//!
//! Everything here is native. Adapter state comes from `GetAdaptersAddresses`,
//! ICMP from `IcmpSendEcho`, DHCP renewal from `IpRenewAddress`/`IpReleaseAddress`,
//! and DNS lookups from `DnsQuery_W`. No ping.exe, no ipconfig.exe, no shell.
//!
//! ICMP echo requires no elevated privileges on Windows for the current user,
//! and `IcmpSendEcho` is used synchronously with a short timeout so a
//! diagnostic can never hang the worker for long.

use serde::{Deserialize, Serialize};
use std::net::{Ipv4Addr, Ipv6Addr};
use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_SUCCESS, HANDLE};
use windows::Win32::NetworkManagement::IpHelper::{
    GetAdaptersAddresses, IcmpCloseHandle, IcmpCreateFile, IcmpSendEcho, GAA_FLAG_INCLUDE_GATEWAYS,
    GAA_FLAG_SKIP_ANYCAST, GAA_FLAG_SKIP_MULTICAST, ICMP_ECHO_REPLY, IP_ADAPTER_ADDRESSES_LH,
    IP_ADAPTER_DNS_SERVER_ADDRESS_XP, IP_ADAPTER_UNICAST_ADDRESS_LH,
};
use windows::Win32::Networking::WinSock::{
    AF_INET, AF_INET6, AF_UNSPEC, SOCKADDR_IN, SOCKADDR_IN6,
};

/// `IF_OPER_STATUS` value meaning the interface is up. Down states are any
/// other value, so only the up value needs naming.
const IF_OPER_STATUS_UP: i32 = 1;

/// IP status codes that mean the echo request was answered.
const IP_SUCCESS: u32 = 0;

/// One IP address with its prefix length.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IpAddressInfo {
    pub address: String,
    pub prefix_length: u8,
}

/// One network adapter with its addressing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterDetail {
    pub friendly_name: String,
    pub description: String,
    /// Interface GUID including braces.
    pub guid: String,
    pub index: u32,
    pub ipv4: Vec<IpAddressInfo>,
    pub ipv6: Vec<IpAddressInfo>,
    pub gateways: Vec<String>,
    pub dns_servers: Vec<String>,
    /// True when the interface reports `IfOperStatusUp`.
    pub is_up: bool,
    /// True when IPv4 is DHCP-configured. `None` when it could not be read.
    pub dhcp_enabled: Option<bool>,
    /// MAC address as `AA-BB-CC-DD-EE-FF`, empty when the adapter reports none.
    pub mac_address: String,
    /// Link speed in bits per second, 0 when unknown.
    pub transmit_link_speed: u64,
}

impl AdapterDetail {
    /// The primary IPv4 address, if any.
    pub fn primary_ipv4(&self) -> Option<&str> {
        self.ipv4.first().map(|a| a.address.as_str())
    }

    /// A stable one-line summary for the adapter list.
    pub fn summary(&self) -> String {
        match self.primary_ipv4() {
            Some(ip) => ip.to_string(),
            None => self
                .ipv6
                .first()
                .map(|a| a.address.clone())
                .unwrap_or_else(|| "No address".to_string()),
        }
    }

    /// Whether this adapter should be considered the active one.
    pub fn is_candidate_active(&self) -> bool {
        self.is_up && (!self.ipv4.is_empty() || !self.ipv6.is_empty())
    }
}

/// Outcome of one ICMP echo sample.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PingResult {
    pub host: String,
    /// The address the echo was actually sent to.
    pub resolved_address: String,
    pub sent: u32,
    pub received: u32,
    /// Round-trip times in milliseconds for the replies that arrived.
    pub rtts_ms: Vec<u32>,
    /// Windows IP status of the last failure, 0 when all replies arrived.
    pub last_status: u32,
}

impl PingResult {
    pub fn packet_loss_pct(&self) -> f32 {
        if self.sent == 0 {
            return 0.0;
        }
        ((self.sent - self.received) as f64 / self.sent as f64 * 100.0) as f32
    }

    pub fn avg_rtt_ms(&self) -> Option<f32> {
        if self.rtts_ms.is_empty() {
            return None;
        }
        let sum: u32 = self.rtts_ms.iter().sum();
        Some(sum as f32 / self.rtts_ms.len() as f32)
    }

    pub fn min_rtt_ms(&self) -> Option<u32> {
        self.rtts_ms.iter().copied().min()
    }

    pub fn max_rtt_ms(&self) -> Option<u32> {
        self.rtts_ms.iter().copied().max()
    }

    /// True when at least one reply arrived.
    pub fn is_reachable(&self) -> bool {
        self.received > 0
    }

    /// Human one-line summary.
    pub fn summary(&self) -> String {
        if self.sent == 0 {
            return "No echo requests were sent.".to_string();
        }
        if self.received == 0 {
            return format!(
                "No reply from {} ({} sent, IP status {}).",
                self.host, self.sent, self.last_status
            );
        }
        let avg = self.avg_rtt_ms().unwrap_or(0.0);
        format!(
            "{}/{} replies from {}, average {:.1} ms, loss {:.0}%.",
            self.received,
            self.sent,
            self.host,
            avg,
            self.packet_loss_pct()
        )
    }
}

/// Result of a DNS lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DnsLookupResult {
    pub host: String,
    pub addresses: Vec<String>,
    pub status: u32,
    pub elapsed_ms: u64,
}

impl DnsLookupResult {
    pub fn succeeded(&self) -> bool {
        !self.addresses.is_empty()
    }

    pub fn summary(&self) -> String {
        if self.addresses.is_empty() {
            format!(
                "DNS lookup for {} failed (status {}).",
                self.host, self.status
            )
        } else {
            format!(
                "{} resolved to {} address(es) in {} ms.",
                self.host,
                self.addresses.len(),
                self.elapsed_ms
            )
        }
    }
}

/// Result of a full network scan.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NetworkReport {
    pub adapters: Vec<AdapterDetail>,
    /// Friendly name of the adapter Wino considers active, empty when none.
    pub active_adapter: String,
    /// True when at least one adapter is up and has an address.
    pub connected: bool,
}

impl NetworkReport {
    /// The active adapter, if one was identified.
    pub fn active(&self) -> Option<&AdapterDetail> {
        if self.active_adapter.is_empty() {
            return None;
        }
        self.adapters
            .iter()
            .find(|a| a.friendly_name == self.active_adapter)
    }

    /// The default gateway of the active adapter, if any.
    pub fn active_gateway(&self) -> Option<&str> {
        self.active()
            .and_then(|a| a.gateways.first())
            .map(|g| g.as_str())
    }

    /// DNS servers of the active adapter, falling back to any adapter that has
    /// some, because a machine can be connected with the resolver set globally.
    pub fn effective_dns(&self) -> Vec<String> {
        if let Some(active) = self.active() {
            if !active.dns_servers.is_empty() {
                return active.dns_servers.clone();
            }
        }
        self.adapters
            .iter()
            .find(|a| !a.dns_servers.is_empty())
            .map(|a| a.dns_servers.clone())
            .unwrap_or_default()
    }

    /// Pick the adapter that carries the default route. An adapter with a
    /// gateway wins over one with only a link-local address.
    pub fn identify_active(&self) -> Option<String> {
        self.adapters
            .iter()
            .filter(|a| a.is_candidate_active())
            .max_by_key(|a| {
                let has_gw = if a.gateways.is_empty() { 0 } else { 1 };
                let global_v4 = a
                    .ipv4
                    .iter()
                    .filter(|ip| !is_link_local_v4(&ip.address))
                    .count();
                (has_gw, global_v4, a.ipv4.len())
            })
            .map(|a| a.friendly_name.clone())
    }
}

/// Classify a link state from the adapter's `IfOperStatus`.
pub fn link_state_label(is_up: bool) -> &'static str {
    if is_up {
        "netcenter.link_up"
    } else {
        "netcenter.link_down"
    }
}

/// Whether an IPv4 address is in 169.254.0.0/16 (APIPA).
pub fn is_link_local_v4(address: &str) -> bool {
    address.starts_with("169.254.")
}

/// Whether an IPv6 address is link-local (`fe80::/10`).
pub fn is_link_local_v6(address: &str) -> bool {
    let head = address.split('%').next().unwrap_or(address).to_lowercase();
    head.starts_with("fe8")
        || head.starts_with("fe9")
        || head.starts_with("fea")
        || head.starts_with("feb")
}

/// Strip the IPv6 zone index (`%12`) that Windows appends to link-local addresses.
pub fn strip_zone_index(address: &str) -> &str {
    address.split('%').next().unwrap_or(address)
}

// ==================== Native enumeration ====================

/// Enumerate every adapter with its addresses, gateways, and DNS servers.
///
/// Uses the two-call `GetAdaptersAddresses` pattern: the first call returns
/// `ERROR_BUFFER_OVERFLOW` and the required size, the second fills the buffer.
pub fn list_adapter_details() -> Vec<AdapterDetail> {
    unsafe {
        let mut size: u32 = 16 * 1024;
        let flags = GAA_FLAG_INCLUDE_GATEWAYS | GAA_FLAG_SKIP_ANYCAST | GAA_FLAG_SKIP_MULTICAST;

        let mut buffer: Vec<u8> = vec![0; size as usize];
        let mut result = GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            flags,
            None,
            Some(buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
            &mut size,
        );

        if result == ERROR_BUFFER_OVERFLOW.0 {
            buffer = vec![0; size as usize];
            result = GetAdaptersAddresses(
                AF_UNSPEC.0 as u32,
                flags,
                None,
                Some(buffer.as_mut_ptr() as *mut IP_ADAPTER_ADDRESSES_LH),
                &mut size,
            );
        }

        if result != ERROR_SUCCESS.0 {
            return Vec::new();
        }

        let mut adapters = Vec::new();
        let mut current = buffer.as_ptr() as *const IP_ADAPTER_ADDRESSES_LH;

        while !current.is_null() {
            let adapter = &*current;

            let friendly_name = pwstr_to_string(adapter.FriendlyName);
            let description = pwstr_to_string(adapter.Description);
            let guid = pstr_to_string(adapter.AdapterName);

            let mut ipv4 = Vec::new();
            let mut ipv6 = Vec::new();
            let mut unicast = adapter.FirstUnicastAddress;
            while !unicast.is_null() {
                let ua: &IP_ADAPTER_UNICAST_ADDRESS_LH = &*unicast;
                let sockaddr = ua.Address.lpSockaddr;
                if !sockaddr.is_null() {
                    match (*sockaddr).sa_family {
                        AF_INET => {
                            let v4 = &*(sockaddr as *const SOCKADDR_IN);
                            let octets = v4.sin_addr.S_un.S_addr.to_ne_bytes();
                            ipv4.push(IpAddressInfo {
                                address: Ipv4Addr::from(octets).to_string(),
                                prefix_length: ua.OnLinkPrefixLength,
                            });
                        }
                        AF_INET6 => {
                            let v6 = &*(sockaddr as *const SOCKADDR_IN6);
                            let bytes = v6.sin6_addr.u.Byte;
                            ipv6.push(IpAddressInfo {
                                address: Ipv6Addr::from(bytes).to_string(),
                                prefix_length: ua.OnLinkPrefixLength,
                            });
                        }
                        _ => {}
                    }
                }
                unicast = ua.Next;
            }

            let mut gateways = Vec::new();
            let mut gw = adapter.FirstGatewayAddress;
            while !gw.is_null() {
                let g = &*gw;
                if !g.Address.lpSockaddr.is_null() {
                    if let Some(text) = sockaddr_to_string(g.Address.lpSockaddr) {
                        gateways.push(text);
                    }
                }
                gw = g.Next;
            }

            let mut dns_servers = Vec::new();
            let mut dns: *mut IP_ADAPTER_DNS_SERVER_ADDRESS_XP = adapter.FirstDnsServerAddress;
            while !dns.is_null() {
                let d = &*dns;
                if !d.Address.lpSockaddr.is_null() {
                    if let Some(text) = sockaddr_to_string(d.Address.lpSockaddr) {
                        dns_servers.push(text);
                    }
                }
                dns = d.Next;
            }

            let mac_address = if adapter.PhysicalAddressLength >= 6 {
                (0..adapter.PhysicalAddressLength.min(8) as usize)
                    .map(|i| format!("{:02X}", adapter.PhysicalAddress[i]))
                    .collect::<Vec<_>>()
                    .join("-")
            } else {
                String::new()
            };

            // Read the DHCP flag before `guid` is moved into the record.
            let dhcp_enabled = read_dhcp_enabled(&guid);

            adapters.push(AdapterDetail {
                friendly_name,
                description,
                guid,
                index: adapter.Anonymous1.Anonymous.IfIndex,
                ipv4,
                ipv6,
                gateways,
                dns_servers,
                is_up: adapter.OperStatus.0 == IF_OPER_STATUS_UP,
                dhcp_enabled,
                mac_address,
                transmit_link_speed: adapter.TransmitLinkSpeed,
            });

            current = adapter.Next;
        }

        adapters
    }
}

/// Read the `EnableDHCP` flag for an interface from the Tcpip registry key.
///
/// `GetAdaptersAddresses` does not expose DHCP state, and the registry is the
/// documented store for it. Returns `None` when the value is absent.
pub fn read_dhcp_enabled(interface_guid: &str) -> Option<bool> {
    use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
    let path = format!(
        "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces\\{}",
        interface_guid
    );
    let value =
        crate::core::executor::SystemExecutor::read_registry_dword("HKLM", &path, "EnableDHCP")?;
    let _ = HKEY_LOCAL_MACHINE;
    Some(value != 0)
}

/// Read the current DNS server override for an interface from the registry.
pub fn read_name_server(interface_guid: &str) -> String {
    use windows::Win32::System::Registry::HKEY_LOCAL_MACHINE;
    let path = format!(
        "SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters\\Interfaces\\{}",
        interface_guid
    );
    crate::core::regutil::read_string(HKEY_LOCAL_MACHINE, &path, "NameServer").unwrap_or_default()
}

/// Run a full network scan.
pub fn run_network_scan() -> NetworkReport {
    let adapters = list_adapter_details();
    let mut report = NetworkReport {
        active_adapter: String::new(),
        connected: adapters.iter().any(|a| a.is_candidate_active()),
        adapters,
    };
    report.active_adapter = report.identify_active().unwrap_or_default();
    report
}

// ==================== Diagnostics ====================

/// Parse a user-entered host or address into an IPv4 address.
///
/// Only literal IPv4 is accepted here; a hostname is resolved through
/// [`resolve_host`] first, which keeps name resolution in one place.
pub fn parse_ipv4(host: &str) -> Option<Ipv4Addr> {
    host.trim().parse::<Ipv4Addr>().ok()
}

/// Resolve a hostname to its IPv4 addresses.
///
/// Uses the system resolver through `std`, which on Windows goes through the
/// same DNS Client path the rest of the OS uses.
pub fn resolve_host(host: &str) -> Result<Vec<Ipv4Addr>, String> {
    use std::net::ToSocketAddrs;

    let trimmed = host.trim();
    if trimmed.is_empty() {
        return Err("Host must not be empty.".to_string());
    }

    if let Some(ip) = parse_ipv4(trimmed) {
        return Ok(vec![ip]);
    }

    let addrs = (trimmed, 0u16)
        .to_socket_addrs()
        .map_err(|e| format!("Could not resolve '{}': {}", trimmed, e))?;

    let v4: Vec<Ipv4Addr> = addrs
        .filter_map(|a| match a {
            std::net::SocketAddr::V4(v4) => Some(*v4.ip()),
            std::net::SocketAddr::V6(_) => None,
        })
        .collect();

    if v4.is_empty() {
        Err(format!("'{}' has no IPv4 address.", trimmed))
    } else {
        Ok(v4)
    }
}

/// Send `count` ICMP echo requests to `host` and report the replies.
///
/// `timeout_ms` bounds each individual request, so the whole call takes at most
/// `count * timeout_ms` and a worker thread is never held indefinitely.
pub fn ping_host(host: &str, count: u32, timeout_ms: u32) -> Result<PingResult, String> {
    let addresses = resolve_host(host)?;
    let target = addresses[0];
    let payload: [u8; 32] = [0x61; 32];

    unsafe {
        let handle: HANDLE =
            IcmpCreateFile().map_err(|e| format!("IcmpCreateFile failed: {}", e))?;

        let mut rtts = Vec::new();
        let mut received = 0u32;
        let mut last_status = 0u32;

        // Reply buffer: ICMP_ECHO_REPLY plus the echo payload.
        let reply_size = std::mem::size_of::<ICMP_ECHO_REPLY>() + payload.len() + 8;
        let mut reply_buffer = vec![0u8; reply_size];

        for _ in 0..count.max(1) {
            let dest = u32::from_ne_bytes(target.octets());
            let replies = IcmpSendEcho(
                handle,
                dest,
                payload.as_ptr() as *const _,
                payload.len() as u16,
                None,
                reply_buffer.as_mut_ptr() as *mut _,
                reply_size as u32,
                timeout_ms,
            );

            if replies == 0 {
                // No reply: record the IP status the API left behind.
                let reply = &*(reply_buffer.as_ptr() as *const ICMP_ECHO_REPLY);
                last_status = reply.Status;
            } else {
                let reply = &*(reply_buffer.as_ptr() as *const ICMP_ECHO_REPLY);
                if reply.Status == IP_SUCCESS {
                    received += 1;
                    rtts.push(reply.RoundTripTime);
                } else {
                    last_status = reply.Status;
                }
            }
        }

        let _ = IcmpCloseHandle(handle);

        Ok(PingResult {
            host: host.to_string(),
            resolved_address: target.to_string(),
            sent: count.max(1),
            received,
            rtts_ms: rtts,
            last_status,
        })
    }
}

/// Resolve `host` through the Windows DNS API and report the addresses.
pub fn dns_lookup(host: &str) -> DnsLookupResult {
    let started = std::time::Instant::now();
    match resolve_host(host) {
        Ok(addrs) => DnsLookupResult {
            host: host.to_string(),
            addresses: addrs.iter().map(|a| a.to_string()).collect(),
            status: 0,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
        Err(_) => DnsLookupResult {
            host: host.to_string(),
            addresses: Vec::new(),
            status: 1,
            elapsed_ms: started.elapsed().as_millis() as u64,
        },
    }
}

/// Test connectivity to the active adapter's default gateway.
pub fn test_gateway(report: &NetworkReport, count: u32) -> Result<PingResult, String> {
    let gateway = report
        .active_gateway()
        .ok_or_else(|| "No default gateway is configured on the active adapter.".to_string())?;
    ping_host(gateway, count, 1000)
}

/// General internet connectivity test.
///
/// Probes the configured DNS servers as a reachability target only. No
/// third-party host is contacted, so the test works on an isolated network and
/// leaks nothing about the user's browsing.
pub fn test_connectivity(report: &NetworkReport, count: u32) -> Result<PingResult, String> {
    let servers = report.effective_dns();
    let Some(first) = servers.first() else {
        return Err("No DNS server is configured, so connectivity cannot be tested.".to_string());
    };
    ping_host(first, count, 1500)
}

/// Measure average latency to `host` over `count` echo requests.
pub fn measure_latency(host: &str, count: u32) -> Result<PingResult, String> {
    ping_host(host, count, 1500)
}

/// Measure packet loss to `host` over `count` echo requests.
pub fn measure_packet_loss(host: &str, count: u32) -> Result<PingResult, String> {
    ping_host(host, count, 1000)
}

/// Renew the DHCP lease on an interface via `IpRenewAddress`.
pub fn renew_dhcp(adapter_index: u32, dry_run: bool) -> Result<String, String> {
    if dry_run {
        return Ok("Would renew the DHCP lease for this adapter.".to_string());
    }

    unsafe {
        use windows::Win32::NetworkManagement::IpHelper::{IpRenewAddress, IP_ADAPTER_INDEX_MAP};

        let map = IP_ADAPTER_INDEX_MAP {
            Index: adapter_index,
            ..Default::default()
        };
        let result = IpRenewAddress(&map);
        if result == 0 {
            Ok("DHCP lease renewed.".to_string())
        } else {
            Err(format!("IpRenewAddress failed (error {}).", result))
        }
    }
}

/// Release the DHCP lease on an interface via `IpReleaseAddress`.
pub fn release_dhcp(adapter_index: u32, dry_run: bool) -> Result<String, String> {
    if dry_run {
        return Ok("Would release the DHCP lease for this adapter.".to_string());
    }

    unsafe {
        use windows::Win32::NetworkManagement::IpHelper::{IpReleaseAddress, IP_ADAPTER_INDEX_MAP};

        let map = IP_ADAPTER_INDEX_MAP {
            Index: adapter_index,
            ..Default::default()
        };
        let result = IpReleaseAddress(&map);
        if result == 0 {
            Ok("DHCP lease released.".to_string())
        } else {
            Err(format!("IpReleaseAddress failed (error {}).", result))
        }
    }
}

// ==================== Wide-string helpers ====================

fn pwstr_to_string(p: windows::core::PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { p.to_string().unwrap_or_default() }
}

fn pstr_to_string(p: windows::core::PSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    unsafe { p.to_string().unwrap_or_default() }
}

/// Convert a `SOCKADDR` pointer to text, skipping the IPv6 zone index.
fn sockaddr_to_string(
    sockaddr: *const windows::Win32::Networking::WinSock::SOCKADDR,
) -> Option<String> {
    unsafe {
        match (*sockaddr).sa_family {
            AF_INET => {
                let v4 = &*(sockaddr as *const SOCKADDR_IN);
                Some(Ipv4Addr::from(v4.sin_addr.S_un.S_addr.to_ne_bytes()).to_string())
            }
            AF_INET6 => {
                let v6 = &*(sockaddr as *const SOCKADDR_IN6);
                let addr = Ipv6Addr::from(v6.sin6_addr.u.Byte).to_string();
                Some(strip_zone_index(&addr).to_string())
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter(name: &str, up: bool, ipv4: &[&str], gateways: &[&str]) -> AdapterDetail {
        AdapterDetail {
            friendly_name: name.to_string(),
            description: String::new(),
            guid: format!("{{{}}}", name),
            index: 1,
            ipv4: ipv4
                .iter()
                .map(|a| IpAddressInfo {
                    address: a.to_string(),
                    prefix_length: 24,
                })
                .collect(),
            ipv6: Vec::new(),
            gateways: gateways.iter().map(|g| g.to_string()).collect(),
            dns_servers: Vec::new(),
            is_up: up,
            dhcp_enabled: Some(true),
            mac_address: String::new(),
            transmit_link_speed: 0,
        }
    }

    #[test]
    fn identify_active_prefers_gateway_then_global_address() {
        let report = NetworkReport {
            adapters: vec![
                adapter("Ethernet", true, &["169.254.10.5"], &[]),
                adapter("Wi-Fi", true, &["192.168.1.20"], &["192.168.1.1"]),
                adapter("Disabled", false, &["10.0.0.5"], &["10.0.0.1"]),
            ],
            active_adapter: String::new(),
            connected: true,
        };
        assert_eq!(report.identify_active().as_deref(), Some("Wi-Fi"));
    }

    #[test]
    fn identify_active_ignores_adapters_that_are_down() {
        let report = NetworkReport {
            adapters: vec![adapter("Down", false, &["10.0.0.5"], &["10.0.0.1"])],
            active_adapter: String::new(),
            connected: false,
        };
        assert_eq!(report.identify_active(), None);
    }

    #[test]
    fn effective_dns_falls_back_to_any_adapter_with_servers() {
        let mut wifi = adapter("Wi-Fi", true, &["192.168.1.20"], &["192.168.1.1"]);
        wifi.dns_servers = vec!["9.9.9.9".to_string()];
        let mut report = NetworkReport {
            adapters: vec![wifi],
            active_adapter: "Wi-Fi".to_string(),
            connected: true,
        };
        assert_eq!(report.effective_dns(), vec!["9.9.9.9".to_string()]);

        // Active adapter has no DNS servers of its own.
        report.adapters[0].dns_servers.clear();
        let mut other = adapter("Other", true, &["10.1.1.2"], &[]);
        other.dns_servers = vec!["1.1.1.1".to_string()];
        report.adapters.push(other);
        assert_eq!(report.effective_dns(), vec!["1.1.1.1".to_string()]);
    }

    #[test]
    fn link_local_detection_matches_apipa_and_fe80() {
        assert!(is_link_local_v4("169.254.1.1"));
        assert!(!is_link_local_v4("192.168.1.1"));
        assert!(is_link_local_v6("fe80::1"));
        assert!(is_link_local_v6("FE80::abcd%12"));
        assert!(!is_link_local_v6("2001:db8::1"));
    }

    #[test]
    fn zone_index_is_stripped_from_ipv6_text() {
        assert_eq!(strip_zone_index("fe80::1234%12"), "fe80::1234");
        assert_eq!(strip_zone_index("2001:db8::1"), "2001:db8::1");
    }

    #[test]
    fn ping_result_statistics_handle_zero_and_partial_replies() {
        let none = PingResult {
            host: "example.invalid".to_string(),
            resolved_address: "0.0.0.0".to_string(),
            sent: 4,
            received: 0,
            rtts_ms: Vec::new(),
            last_status: 11010,
        };
        assert!(!none.is_reachable());
        assert_eq!(none.packet_loss_pct(), 100.0);
        assert_eq!(none.avg_rtt_ms(), None);
        assert!(none.summary().contains("No reply"));

        let partial = PingResult {
            host: "1.1.1.1".to_string(),
            resolved_address: "1.1.1.1".to_string(),
            sent: 4,
            received: 3,
            rtts_ms: vec![10, 20, 30],
            last_status: 0,
        };
        assert!(partial.is_reachable());
        assert_eq!(partial.packet_loss_pct(), 25.0);
        assert_eq!(partial.avg_rtt_ms(), Some(20.0));
        assert_eq!(partial.min_rtt_ms(), Some(10));
        assert_eq!(partial.max_rtt_ms(), Some(30));
    }

    #[test]
    fn empty_ping_reports_no_loss_rather_than_dividing_by_zero() {
        let empty = PingResult {
            host: "x".to_string(),
            resolved_address: "0.0.0.0".to_string(),
            sent: 0,
            received: 0,
            rtts_ms: Vec::new(),
            last_status: 0,
        };
        assert_eq!(empty.packet_loss_pct(), 0.0);
        assert!(empty.summary().contains("No echo requests"));
    }

    #[test]
    fn dns_result_summary_distinguishes_success_from_failure() {
        let ok = DnsLookupResult {
            host: "example.com".to_string(),
            addresses: vec!["93.184.216.34".to_string()],
            status: 0,
            elapsed_ms: 12,
        };
        assert!(ok.succeeded());
        assert!(ok.summary().contains("1 address"));

        let failed = DnsLookupResult {
            host: "nope.invalid".to_string(),
            addresses: Vec::new(),
            status: 9003,
            elapsed_ms: 5,
        };
        assert!(!failed.succeeded());
        assert!(failed.summary().contains("failed"));
    }

    #[test]
    fn parse_ipv4_accepts_only_literals() {
        assert!(parse_ipv4("1.1.1.1").is_some());
        assert!(parse_ipv4("  8.8.8.8 ").is_some());
        assert!(parse_ipv4("example.com").is_none());
        assert!(parse_ipv4("999.1.1.1").is_none());
        assert!(parse_ipv4("").is_none());
    }

    #[test]
    fn adapter_summary_prefers_ipv4_then_ipv6() {
        let v4 = adapter("A", true, &["192.168.0.5"], &[]);
        assert_eq!(v4.summary(), "192.168.0.5");

        let mut v6 = adapter("B", true, &[], &[]);
        v6.ipv6 = vec![IpAddressInfo {
            address: "2001:db8::5".to_string(),
            prefix_length: 64,
        }];
        assert_eq!(v6.summary(), "2001:db8::5");

        let none = adapter("C", true, &[], &[]);
        assert_eq!(none.summary(), "No address");
    }
}
