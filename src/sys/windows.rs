use crate::{Error, Result, Wifi};
use regex::Regex;
use std::vec::Vec;
use tokio::process::Command;

/// Returns a list of WiFi hotspots in your area - (Windows) uses `netsh`
pub async fn scan() -> Result<Vec<Wifi>> {
    let output = Command::new("netsh.exe")
        .args(&["wlan", "show", "networks", "mode=Bssid"])
        .output()
        .await
        .map_err(|_| Error::CommandNotFound)?;

    let data = String::from_utf8_lossy(&output.stdout);
    parse_netsh(&data)
}

fn parse_netsh(network_list: &str) -> Result<Vec<Wifi>> {
    let mut wifis = Vec::new();

    // Regex for matching split, SSID and MAC, since these aren't pulled directly
    let split_regex = Regex::new("\nSSID").map_err(|_| Error::SyntaxRegexError)?;
    let ssid_regex = Regex::new("^ [0-9]* : ").map_err(|_| Error::SyntaxRegexError)?;
    let mac_regex = Regex::new("[a-fA-F0-9:]{17}").map_err(|_| Error::SyntaxRegexError)?;

    for block in split_regex.split(network_list) {
        let mut wifi_macs = Vec::new();
        let mut wifi_ssid = String::new();
        let mut wifi_channels = Vec::new();
        let mut wifi_rssi = Vec::new();
        let mut wifi_security = String::new();

        for line in block.lines() {
            if let Some(ssid_match) = ssid_regex.find(line) {
                wifi_ssid = ssid_match
                    .as_str()
                    .split(":")
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .to_string();
            } else if let Some(auth_line) = line.split(":").next() {
                wifi_security = auth_line.trim().to_string();
            } else if line.contains("BSSID") {
                if let Some(captures) = mac_regex.captures(line) {
                    // Default to an empty string if no match is found
                    let mac = captures.get(0).map_or("", |m| m.as_str());
                    wifi_macs.push(mac.to_string());
                }
            } else if line.contains("Signal") {
                if let Some(percent) = line.split(":").nth(1) {
                    let rssi = percent
                        .trim()
                        .replace("%", "")
                        .parse::<i32>()
                        .unwrap_or(-100);
                    wifi_rssi.push(rssi / 2 - 100);
                }
            } else if line.contains("Channel") {
                if let Some(channel) = line.split(":").nth(1) {
                    wifi_channels.push(channel.trim().to_string());
                }
            }
        }

        for (mac, channel, rssi) in izip!(wifi_macs, wifi_channels, wifi_rssi) {
            wifis.push(Wifi {
                mac: mac.as_str().to_string(),
                ssid: wifi_ssid.to_string(),
                channel: channel.to_string(),
                signal_level: rssi.to_string(),
                security: wifi_security.to_string(),
            });
        }
    }

    Ok(wifis)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn should_parse_netsh() {
        use std::fs;

        // Note: formula for % to dBm is (% / 100) - 100
        let expected = vec![
            Wifi {
                mac: "ab:cd:ef:01:23:45".to_string(),
                ssid: "Vodafone Hotspot".to_string(),
                channel: "6".to_string(),
                signal_level: "-92".to_string(),
                security: "Open".to_string(),
            },
            Wifi {
                mac: "ab:cd:ef:01:23:45".to_string(),
                ssid: "Vodafone Hotspot".to_string(),
                channel: "6".to_string(),
                signal_level: "-73".to_string(),
                security: "Open".to_string(),
            },
            Wifi {
                mac: "ab:cd:ef:01:23:45".to_string(),
                ssid: "EdaBox".to_string(),
                channel: "11".to_string(),
                signal_level: "-82".to_string(),
                security: "WPA2-Personal".to_string(),
            },
            Wifi {
                mac: "ab:cd:ef:01:23:45".to_string(),
                ssid: "FRITZ!Box 2345 Cable".to_string(),
                channel: "1".to_string(),
                signal_level: "-50".to_string(),
                security: "WPA2-Personal".to_string(),
            },
        ];

        // Load test fixtures
        let fixture = fs::read_to_string("tests/fixtures/netsh/netsh01_windows81.txt").unwrap();

        let result = parse_netsh(&fixture).unwrap();
        assert_eq!(expected[0], result[0]);
        assert_eq!(expected[1], result[1]);
        assert_eq!(expected[2], result[2]);
        assert_eq!(expected[3], result[3]);
    }
}
