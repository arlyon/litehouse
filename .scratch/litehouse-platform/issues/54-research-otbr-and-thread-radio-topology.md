# Research the OTBR and Thread-radio boundary

Type: research
Mode: AFK
Status: resolved
Resolved: 2026-07-16
Blocked by: 30, 42
Map: [Find the route to Litehouse's first connected release](../map.md)

## Decision

**Keep Thread routing entirely outside Litehouse.** The supported topology should be:

```text
Matter controller / other LAN application
             │ IPv6 unicast + DNS-SD/mDNS on the LAN
             ▼
operator-managed OTBR service or certified border-router appliance
             │ Spinel over local UART/USB (reference topology)
             ▼
dedicated 802.15.4 RCP and antenna
```

For a host-built reference deployment, recommend a **separate, operator-managed, digest-pinned OTBR container with host networking and a dedicated USB RCP**. Recommend an **EFR32MG24-class USB RCP with a purpose-built/external antenna** for the deployment baseline, and retain an **nRF52840 Dongle** as the cheapest upstream-proven development and recovery baseline. A vendor-managed Ethernet/PoE border-router appliance is a credible placement alternative; a raw RCP-over-TCP endpoint is not the default because current upstream OpenThread POSIX documents native Spinel transports as UART, SPI, and a `forkpty` subprocess—not TCP.[OT-RADIO-URL][SML-ETH]

This decision does **not** add Thread or Matter to Litehouse V1. The current Litehouse container explicitly has no mDNS, host network, `NET_ADMIN`, device pass-through, or additional writable state.[LH30] A future Matter integration must separately decide where its Matter controller, fabric credentials, commissioning UI, DNS-SD, and Thread credential exchange live.

## OTBR is not a Matter controller

A Thread Border Router connects a Thread IPv6 stub network to Wi-Fi/Ethernet, configures routes/prefixes, and proxies service discovery. OTBR additionally implements NAT64, DHCPv6 Prefix Delegation, and the Thread Border Agent.[OTBR-GUIDE][MATTER-IPV6] Thread Group explicitly describes Thread as application-layer agnostic and Matter as the application protocol running over Thread, Wi-Fi, or Ethernet.[TG13]

A Matter Commissioner/controller instead assigns Matter fabric credentials, performs PASE and attestation, obtains and installs Node Operational Certificates, provisions the operational network, performs operational DNS-SD discovery, establishes CASE, and controls Matter data-model endpoints.[MATTER-COMMISSION]

Therefore:

- OTBR can provide IPv6 reachability to a Matter-over-Thread node and advertise its registered `_matter._tcp` service, but it does not own a Matter fabric, commission Matter operational credentials, implement CASE/Interaction Model control, or become Litehouse's Matter controller.[TG13-WP][MATTER-COMMISSION]
- Thread external commissioning through the OTBR Border Agent is a Thread-network function, not Matter commissioning.
- Litehouse must never infer “OTBR present” means “Matter support present.” A distinct controller implementation and trust/state design are required.

## Why OTBR must be a separate operator-managed service/container

The upstream OTBR container contract is structurally incompatible with Litehouse's non-root, read-only, single-container contract:

| OTBR requirement/effect | Litehouse contract conflict |
|---|---|
| Upstream runs with `--network=host` so OTBR can access infrastructure interfaces directly. | Litehouse uses an ordinary bridge and forbids host networking and ambient multicast.[OTBR-DOCKER][LH30] |
| Upstream adds `NET_ADMIN`, maps `/dev/net/tun`, and maps the RCP serial device. | Litehouse drops all capabilities and forbids TUN, serial devices, and all device pass-through.[OTBR-DOCKER][LH30] |
| Host setup enables IPv4/IPv6 forwarding and sets `accept_ra=2` plus `accept_ra_rt_info_max_plen=64` on the infrastructure interface. | These are host-global routing policy changes; Litehouse is not allowed to mutate the host.[OTBR-HOST][LH30] |
| OTBR creates `wpan0`, IPv6 routes/RAs, ipsets, `ip6tables` ingress rules, and IPv4 mangle/NAT/filter rules for NAT64. With host networking these are in the host network namespace. | Litehouse owns no host firewall, routing, NAT, or raw-network policy.[OTBR-RUN][LH30] |
| OTBR runs mDNSResponder and publishes/consumes link-local multicast on the adjacent LAN. | Litehouse V1 explicitly forbids mDNS, multicast, and broadcast.[OTBR-RUN][LH30] |
| The upstream image has no non-root `USER`; `/init` starts as root. Its service creates `/data/thread`, creates `/var/lib/thread` as a symlink, mutates firewall state, and uses s6 runtime state. | Litehouse is UID/GID `65532`, has a read-only root filesystem, one application process, and only its own data/tmpfs paths writable.[OTBR-IMAGE][OTBR-RUN][LH30] |
| OTBR persists its own Thread dataset/state at host `/var/lib/otbr` mounted as `/data`. | Litehouse permits exactly one application volume at `/var/lib/litehouse`; OTBR state has a different owner, schema, backup, and recovery lifecycle.[OTBR-DOCKER][LH30] |
| OTBR and RCP firmware have coupled Spinel compatibility and hardware recovery needs. | Litehouse image rollback must not flash hardware or couple its release cadence to privileged infrastructure.[OT-SPINEL][LH30] |

Putting both processes in one container would either break OTBR or permanently weaken every Litehouse invariant. A sidecar sharing Litehouse's sandbox is also insufficient: OTBR needs a different network namespace, capabilities, devices, persistence, lifecycle, and failure domain. The operator must install, configure, update, back up, firewall, and recover OTBR. Litehouse should receive no OTBR admin socket, REST API, serial device, container socket, or authority to restart/reflash it.

## Thread 1.3+ acceptance boundary

Thread 1.3 standardized border-router IPv6 reachability and DNS-based service discovery needed by Matter: OMR prefixes and routes across the adjacent infrastructure link; SRP service registration on Thread; an Advertising Proxy from SRP to LAN mDNS; and a Discovery Proxy from Thread DNS queries to LAN mDNS. It also standardized TCP requirements for constrained Thread links.[TG13-WP]

Use a **current pinned OTBR/OpenThread release built for Thread 1.4 (or later), with all Thread 1.3 border-routing behavior enabled**. Current OpenThread CMake defaults `OT_THREAD_VERSION` to 1.4, while OTBR enables border routing, SRP server, SRP advertising proxy, DNS-SD discovery proxy, mDNS, and the border agent.[OT-OPTIONS][OTBR-CMAKE] Acceptance must verify behavior, not merely the SoC name or a marketing “Thread 1.3” label:

1. RCP initializes without bypassing the compatibility check and reports all required radio capabilities.
2. OTBR attaches, creates `wpan0`, forms or restores the intended dataset, and survives restart without forming a second network.
3. A LAN host receives the advertised OMR route and has bidirectional IPv6 reachability to a Thread node.
4. A Thread SRP registration appears through mDNS on the adjacent LAN; a Thread node can discover an allowed LAN service through the Discovery Proxy.
5. `_meshcop._udp` is visible only on intended LAN interfaces; management REST/web listeners remain loopback or disabled.
6. NAT64 is tested only if enabled; it is not a substitute for normal IPv6 routing and is not part of the Matter-controller role.
7. Certification claims are made only for a certified complete product/configuration. A supported or Thread-Certified Component radio does not by itself certify the assembled host, image, RCP firmware, antenna, and network deployment.

## Host networking, discovery, firewall, and NAT64

The operator-owned host must provide:

- Linux IPv6 and TUN support; IPv4 and IPv6 forwarding; RA acceptance while forwarding on the selected infrastructure interface; and an iptables/ipset-compatible firewall path matching the pinned OTBR image.[OTBR-HOST][OTBR-IMAGE]
- One explicitly selected Ethernet or Wi-Fi adjacent infrastructure interface. OTBR must not attach to every host interface, VPN, guest VLAN, or container bridge.
- Link-local mDNS on UDP 5353 (including IPv6 `ff02::fb`) on that interface, IPv6 Neighbor Discovery/Router Advertisement traffic, and the routes needed for the OMR prefix. OTBR source publishes the Border Agent as `_meshcop._udp`; Thread 1.3 uses SRP plus Advertising/Discovery proxies rather than flooding mDNS into Thread.[OTBR-MESHCOP][TG13-WP]
- Firewall review after every OTBR update. The current upstream container creates a dedicated IPv6 ingress chain and ipsets, then adds IPv4 mark/MASQUERADE/filter rules for NAT64; those changes must not silently override the host's intended nftables/iptables policy.[OTBR-RUN]

NAT64 is optional egress infrastructure for IPv6-only Thread nodes reaching IPv4 resources. OpenThread publishes a dynamically generated `/96` NAT64 prefix in Thread Network Data, translates TCP/UDP/ICMP, and can synthesize IPv6 answers from IPv4 DNS records; it does not use the well-known `64:ff9b::/96` prefix in this mode.[OTBR-NAT64] Do not use NAT64 for LAN-to-Thread Matter reachability, expose temporary translated IPv4 addresses as device identity, or assume it permits unsolicited IPv4 ingress.

## USB RCP versus Ethernet/network radio

| Concern | Local USB/UART RCP — recommended reference | Ethernet/PoE appliance or network RCP |
|---|---|---|
| Upstream fit | Direct upstream radio URL and documented Docker device mapping. | A complete appliance can hide this boundary; a raw TCP serial endpoint needs vendor integration or a `forkpty` bridge because upstream does not document a native TCP radio URL.[OT-RADIO-URL] |
| Privilege | One stable serial device plus `/dev/net/tun`; still requires separate privileged OTBR. | No radio device mapping into the host if OTBR runs on the appliance; a raw network RCP adds an IP/port and network ACL. |
| Placement/RF | Limited by a reliable USB cable, but simple and low-latency. Use a short shielded USB 2 extension or purpose-built external antenna to move the radio away from the host. | Ethernet/PoE can place the radio centrally and away from a noisy server; it adds switch, DHCP/static-address, PoE, appliance-OS, and remote-update failure domains. |
| Failure/security | Physical local link; recovery can use a spare/reflash at the host. | Treat an undocumented or unauthenticated RCP TCP stream as a trusted-infrastructure protocol: isolate it on a management VLAN/ACL and do not route it over the Internet/VPN merely because a vendor supports that. |
| Update ownership | Operator pins OTBR and RCP firmware as one tested pair. | Vendor may own OTBR, bridge, RCP, and OTA as one appliance; verify signed updates, rollback, export/restore, and local recovery before support. |

SMLIGHT supplies concrete first-party proof that both shapes exist: its EFR32MG24 SLZB-07Mg24 is a USB/UART device with an external antenna and autoflash support; its EFR32MG24 SLZB-06Mg24U provides Ethernet/PoE, an external antenna, remote radio update, and an OTBR role. Its older first-party setup guide also demonstrates an RCP network endpoint configured as `IP:6638`.[SML-USB][SML-ETH][SML-NET] This proves representative hardware, not upstream compatibility or certification; admit an exact hardware revision only after the acceptance suite above.

## Radio and RF recommendation

### Preferred deployment class: dedicated EFR32MG24 USB RCP

Why:

- Silicon Labs documents OpenThread support, 802.15.4 OQPSK sensitivity around -105.4 dBm, and up to 19.5 dBm transmit power for EFR32MG24; the device also exposes a Wi-Fi coexistence signal-identification mechanism.[SIL-MG24][SIL-MG24-DS]
- Credible packaged hardware exists with USB/UART, external antennas, firmware flashing, and stated Thread 1.3 support, rather than requiring Litehouse to design an RF board.[SML-USB]
- It leaves more RF/link-budget and coexistence headroom than the reference nRF52840 USB dongle, whose first-party brief specifies an integrated PCB antenna and maximum 8 dBm output.[NRF-DONGLE]

Qualifiers: purchase and validate one exact SKU/hardware revision; flash **dedicated OpenThread RCP firmware**, not Zigbee coordinator, NCP, multiprotocol/MultiPAN, or an all-in-one Matter image. A first-party Home Assistant radio vendor reports that simultaneous Zigbee/Thread MultiPAN was inconsistent and caused stability issues, supporting the conservative one-radio/one-protocol boundary.[HA-ZBT2]

### Development/recovery baseline: Nordic nRF52840 Dongle

Keep nRF52840 as the upstream-known baseline, not the preferred coverage product. OpenThread's official NAT64 codelab explicitly uses two nRF52840 USB Dongles, one as the RCP, and Nordic's current co-processor sample supports the `nrf52840dongle/nrf52840` target, serial Spinel, Thread 1.4 including 1.3, and reboot-to-bootloader.[OTBR-NAT64][NRF-RCP] It is inexpensive, directly USB-connected, and easy to reproduce, but its integrated PCB antenna and 8 dBm maximum make enclosure/placement more consequential.[NRF-DONGLE]

### Antenna and coexistence rules

- Do not place a 2.4 GHz dongle against the host chassis, SSD, USB 3 hub, Wi-Fi AP, or other 2.4 GHz radio. The USB-IF-hosted Intel white paper shows USB 3 noise in 2.4–2.5 GHz can reduce receiver sensitivity and identifies shielding and antenna placement as mitigations.[USB3-RFI]
- Prefer an external/optimized antenna with strain relief and enough separation to clear the chassis. Set regulatory region and channel power; antenna gain plus conducted power must remain within local EIRP limits. OpenThread's radio URL exposes `region`, per-channel maximum power, coexistence enablement, and external-FEM LNA gain for this reason.[OT-RADIO-URL]
- Coordinate Thread and 2.4 GHz Wi-Fi channels during installation. Hardware coexistence/PTA helps only when the radio and colocated Wi-Fi design are actually wired and configured for it; an EFR32 capability on paper does not create coexistence across separate USB devices.
- Use one dedicated radio for Thread. Do not share it with Zigbee or BLE merely because the SoC is “multiprotocol.”

## Firmware compatibility, update, and recovery contract

The RCP is not a generic serial modem. OpenThread keeps the Thread core on the host and only a minimal MAC/controller on the radio, with Spinel between them.[OT-RCP] Spinel defines an RCP API version range; the host queries the RCP's API/minimum-host versions and required capabilities and fails when they are incompatible. Current source defines API version 11 and minimum host-supported version 4; those values are release-specific, not a permanent compatibility promise.[OT-SPINEL][OT-SPINEL-CHECK]

Operator procedure:

1. Record exact OTBR image digest/source revision, OpenThread host revision/build options, RCP firmware artifact digest, board SKU/revision, bootloader/flasher, baud/flow control, radio region/power, and infrastructure interface.
2. Build or obtain RCP firmware from the same release family or a vendor matrix explicitly tested against that host. Never set `skip-rcp-compatibility-check` in production.[OT-RADIO-URL]
3. Before update, export the operational Thread dataset through the supported operator tooling, stop OTBR, and make a stopped copy of OTBR's `/data`; never store it in Litehouse's volume.
4. Update one layer at a time, restore service, then run API/capability, attach, IPv6, SRP/mDNS, commissioning-advertisement, and optional NAT64 checks. Roll back before changing the other layer.
5. Keep a known-good RCP firmware image, flashing tool, and preferably a preflashed spare. Nordic's sample exposes reboot-to-bootloader on its dongle; SMLIGHT documents USB autoflash/remote radio update, but remote update is not a substitute for a physical recovery path.[NRF-RCP][SML-USB][SML-ETH]
6. Fail closed on unknown RCP identity/version, repeated resets, capability loss, wrong dataset, or unexpected new Thread network. OTBR unavailability degrades Thread connectivity; it must not make Litehouse itself unready.

## Implementation gates

Before Litehouse documents Thread support, require a disposable, non-production proof with no real household credentials:

- exact USB EFR32MG24 SKU and nRF52840 fallback, both on the same pinned OTBR host build;
- cold boot, restart, unplug/replug, incompatible-firmware rejection, reflash, spare replacement, and stopped-state restore;
- Thread 1.3+ IPv6 route and bidirectional reachability, SRP Advertising Proxy, Discovery Proxy, `_meshcop._udp`, and optional NAT64 tests;
- firewall diff before/after start/stop/update, confirming no unintended interface, default-route, or management exposure;
- RF placement/channel test near active Wi-Fi and USB 3 traffic;
- proof that Litehouse still runs unchanged as non-root, read-only, bridged, capability-free, device-free, and with only `/var/lib/litehouse` writable.

## Primary sources

### Litehouse authoritative boundary

- [LH30] [Issue 30: non-root/read-only single-container contract](30-define-container-and-butane-contract.md)

### OpenThread and OTBR

- [OTBR-GUIDE] [Official OTBR guide and functions](https://openthread.io/guides/border-router)
- [OTBR-DOCKER] [Official Docker install/run contract](https://openthread.io/guides/border-router/build-docker)
- [OTBR-HOST] [Pinned host sysctl setup](https://github.com/openthread/ot-br-posix/blob/69f03f0390dfa69837a8bd421da7acb33cb5d546/etc/docker/border-router/setup-host)
- [OTBR-IMAGE] [Pinned upstream runtime image](https://github.com/openthread/ot-br-posix/blob/69f03f0390dfa69837a8bd421da7acb33cb5d546/etc/docker/border-router/Dockerfile)
- [OTBR-RUN] [Pinned service startup, persistence, firewall, NAT, and listeners](https://github.com/openthread/ot-br-posix/blob/69f03f0390dfa69837a8bd421da7acb33cb5d546/etc/docker/border-router/rootfs/etc/s6-overlay/s6-rc.d/otbr-agent/run)
- [OTBR-CMAKE] [Pinned OTBR OpenThread feature configuration](https://github.com/openthread/ot-br-posix/blob/69f03f0390dfa69837a8bd421da7acb33cb5d546/third_party/openthread/CMakeLists.txt)
- [OTBR-MESHCOP] [Pinned Border Agent `_meshcop._udp` publication](https://github.com/openthread/ot-br-posix/blob/69f03f0390dfa69837a8bd421da7acb33cb5d546/src/border_agent/border_agent.cpp#L79-L86)
- [OTBR-NAT64] [Official NAT64 behavior and nRF52840 hardware codelab](https://openthread.io/codelabs/openthread-border-router-nat64)
- [OT-RCP] [Official RCP architecture](https://openthread.io/platforms/co-processor#radio_co-processor_rcp)
- [OT-RADIO-URL] [Pinned POSIX Spinel transports, RF settings, and compatibility switch](https://github.com/openthread/openthread/blob/27e86ae0f001e8885e663393e2570230e00a3ea0/src/posix/platform/radio_url.cpp#L45-L130)
- [OT-SPINEL] [Pinned RCP API compatibility rules and current values](https://github.com/openthread/openthread/blob/27e86ae0f001e8885e663393e2570230e00a3ea0/src/lib/spinel/spinel.h#L313-L432)
- [OT-SPINEL-CHECK] [Pinned host API/capability checks](https://github.com/openthread/openthread/blob/27e86ae0f001e8885e663393e2570230e00a3ea0/src/lib/spinel/radio_spinel.cpp#L228-L315)
- [OT-OPTIONS] [Pinned OpenThread default Thread version](https://github.com/openthread/openthread/blob/27e86ae0f001e8885e663393e2570230e00a3ea0/etc/cmake/options.cmake#L313-L324)

### Thread Group and Matter

- [TG13-WP] [Thread Group: Thread 1.3.0 Features white paper](https://www.threadgroup.org/Portals/0/documents/support/Thread1.3.0WhitePaper_07192022_3990_1.pdf)
- [TG13] [Thread Group: Thread 1.3.0 and Matter/application-layer separation](https://threadgroup.org/Newsroom/Blog/thread-130-brings-unprecedented-matter-enablement-new-levels-of-robustness-to-iot-for-smart-homes-and-buildings)
- [MATTER-IPV6] [Public Matter primer: Thread BR routing and DNS-SD responsibilities](https://developers.home.google.com/matter/primer/thread-and-ipv6)
- [MATTER-COMMISSION] [Public Matter primer: Commissioner/fabric commissioning responsibilities](https://developers.home.google.com/matter/primer/commissioning)

### First-party silicon and hardware evidence

- [NRF-DONGLE] [Nordic nRF52840 Dongle product brief](https://www.nordicsemi.com/-/media/Software-and-other-downloads/Product-Briefs/nRF52840-Dongle-product-brief.pdf)
- [NRF-RCP] [Pinned Nordic Thread co-processor sample](https://github.com/nrfconnect/sdk-nrf/blob/8941f656513f57ff5f2b7122d1b77692771a1b96/samples/openthread/coprocessor/README.rst)
- [SIL-MG24] [Silicon Labs EFR32MG24 product page](https://www.silabs.com/wireless/zigbee/efr32mg24-series-2-socs)
- [SIL-MG24-DS] [Silicon Labs EFR32MG24 data sheet](https://www.silabs.com/documents/public/data-sheets/efr32mg24-datasheet.pdf)
- [SML-USB] [SMLIGHT SLZB-07Mg24 USB hardware and update proof](https://smlight.tech/products/slzb-07mg24)
- [SML-ETH] [SMLIGHT SLZB-06Mg24U Ethernet/PoE OTBR appliance proof](https://smlight.tech/products/slzb-06mg24u)
- [SML-NET] [SMLIGHT network Thread-RCP setup proof](https://smlight.tech/manual/slzb-06/guide/thread-matter/)
- [HA-ZBT2] [Nabu Casa/Home Assistant Connect ZBT-2 hardware and dedicated-radio rationale](https://www.home-assistant.io/connect/zbt-2/)
- [USB3-RFI] [USB-IF-hosted Intel white paper: USB 3.0 interference at 2.4 GHz](https://www.usb.org/sites/default/files/327216.pdf)
