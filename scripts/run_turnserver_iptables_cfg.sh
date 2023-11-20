#!/usr/bin/env bash
# vboxnet0 -> internal network(behind the NAT)
# wlp0s20f3 -> external network(public network)
external_interface="wlp0s20f3"
internal_interface="vboxnet0"
internal_vm_ip="192.168.56.101:3478"
# Loading necessary kernel modules
modprobe iptable_nat
modprobe ip_tables
modprobe ip_conntrack
modprobe ip_conntrack_irc
modprobe ip_conntrack_ftp
echo 1 > /proc/sys/net/ipv4/ip_forward

# Allow traffic on the loopback interface
iptables -A INPUT -i lo -j ACCEPT
iptables -A OUTPUT -o lo -j ACCEPT

# Allow incoming UDP traffic on port 3478 from $external_interface to $internal_interface
iptables -A FORWARD -i $external_interface -o $internal_interface -p udp --dport 3478 -m state --state ESTABLISHED,RELATED -j ACCEPT

# iptables rules for source NAT (MASQUERADE)
iptables -t nat -A POSTROUTING -o $external_interface -j MASQUERADE

# Port forwarding for UDP 3478 (adjust IP as needed)
iptables -t nat -A PREROUTING -i $external_interface -p udp --dport 3478 -j DNAT --to-destination $internal_vm_ip
iptables -A FORWARD -i $external_interface -p udp --dport 3478 -d  $internal_vm_ip -j ACCEPT

# Allow forwarding from $internal_interface to $external_interface
iptables -A FORWARD -i $internal_interface -o $external_interface -j ACCEPT

# Reject all other forwarding from $external_interface to $internal_interface
#iptables -A FORWARD -i $external_interface -o $internal_interface -j REJECT

# Reject all forwarding from $internal_interface to $external_interface
#iptables -A FORWARD -i $internal_interface -o $external_interface -j REJECT
# Reject all forwarding from $internal_interface to $external_interface, except UDP 3478
iptables -A FORWARD -i $internal_interface -o $external_interface -p udp --dport 3478 -j ACCEPT
