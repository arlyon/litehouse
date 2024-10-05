# cockpit

The signalling server for Litehouse. Facilitates direct peer-to-peer connections between devices
over WebRTC, bypassing NAT and firewalls.

## Design

The server is designed to be as simple as possible. Litehouse instances send some basic metadata
(their auth key and identifier) to the server and initiate long polling. When a client connects,
it sends an SDP offer to the server, which the server forwards to the litehouse instance. The
instance then sends an offer back to the client, and they establish a direct peer connection.

If a client connects, and there is no server waiting for a connection, the connection is held
until a server is available.
