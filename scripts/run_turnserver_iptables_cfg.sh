#!/usr/bin/env bash
# vboxnet0 -> internal network(behind the NAT)
# wlp0s20f3 -> external network(public network)
#external_interface="wlp0s20f3"
#internal_interface="vboxnet0"
#internal_vm_ip="192.168.56.101"
NUMBER_OF_ARGS=3

# Check if at least three arguments are provided
if [ $# -ne $NUMBER_OF_ARGS ]; then
  echo "Usage: $0 <external interface> <internal interface> <internal vm ip>"
  exit 1
fi

EXTERNAL_INTERFACE="$1"
INTERNAL_INTERFACE="$2"
INTERNAL_VM_IP="$3"
TURNSERVER_PORT="3478"


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

# Allow incoming UDP traffic on port 3478 from "$EXTERNAL_INTERFACE" to "$INTERNAL_INTERFACE"
iptables -A FORWARD -i "$EXTERNAL_INTERFACE" -o "$INTERNAL_INTERFACE" -p udp --dport "$TURNSERVER_PORT" -m state --state ESTABLISHED,RELATED -j ACCEPT

# iptables rules for source NAT (MASQUERADE)
iptables -t nat -A POSTROUTING -o "$EXTERNAL_INTERFACE" -j MASQUERADE

# Port forwarding for UDP 3478 (adjust IP as needed)
iptables -t nat -A PREROUTING -i "$EXTERNAL_INTERFACE" -p udp --dport "$TURNSERVER_PORT" -j DNAT --to-destination "$INTERNAL_VM_IP"
iptables -A FORWARD -i "$EXTERNAL_INTERFACE" -p udp --dport "$TURNSERVER_PORT" -d  "$INTERNAL_VM_IP" -j ACCEPT

# Allow forwarding from "$INTERNAL_INTERFACE" to "$EXTERNAL_INTERFACE"
iptables -A FORWARD -i "$INTERNAL_INTERFACE" -o "$EXTERNAL_INTERFACE" -j ACCEPT

# Reject all other forwarding from "$EXTERNAL_INTERFACE" to "$INTERNAL_INTERFACE"
#iptables -A FORWARD -i "$EXTERNAL_INTERFACE" -o "$INTERNAL_INTERFACE" -j REJECT

# Reject all forwarding from "$INTERNAL_INTERFACE" to "$EXTERNAL_INTERFACE"
#iptables -A FORWARD -i "$INTERNAL_INTERFACE" -o "$EXTERNAL_INTERFACE" -j REJECT
# Reject all forwarding from "$INTERNAL_INTERFACE" to "$EXTERNAL_INTERFACE", except UDP 3478
iptables -A FORWARD -i "$INTERNAL_INTERFACE" -o "$EXTERNAL_INTERFACE" -p udp --dport "$TURNSERVER_PORT" -j ACCEPT
