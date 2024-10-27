Cockpit is the webtrc signalling server that powers litehouse. An instance of litehouse
may point to a cockpit broker which facilitates a direct peer-to-peer connection between
the web interface and the instance itself.

The signaling server is designed to require minimal resources and as such, is free to use.
The service does not store your instance's password or any other sensitive information either
in memory or at rest. It is entirely stateless.

To prevent abuse, the confirmation APIs (finalize, and reject) are rate-limited per IP.
These two APIs

# Brokering protocol

The brokering protocol in Cockpit involves several steps to establish a direct peer-to-peer WebRTC connection between a user and a Litehouse instance. Here's a user-friendly explanation of the process:

1. **Instance Connection via SSE**: The Litehouse instance connects to the Cockpit server using Server-Sent Events (SSE). This connection allows the instance to receive real-time updates and offers from the server.
2. **User Sends an Offer**: The user initiates a connection by sending an offer to the Cockpit server. This offer contains the necessary WebRTC session description to start the connection process.
3. **Offer Forwarding**: The Cockpit server forwards the user's offer to the connected Litehouse instance. This step ensures that the instance receives the offer and can respond accordingly.
4. **Instance Replies**: The Litehouse instance processes the received offer and replies with either an acceptance or rejection. If the instance accepts the offer, it sends back a counter-offer to complete the WebRTC connection. If the instance rejects the offer, the connection attempt is terminated.
5. **Connection Finalization**: If the instance accepts the offer, the Cockpit server finalizes the connection by exchanging the necessary WebRTC session descriptions and ICE candidates between the user and the instance. This step establishes the direct peer-to-peer connection.
6. **Connection Rejection**: If the instance rejects the offer, the Cockpit server informs the user that the connection attempt was unsuccessful. The user can then decide to retry or terminate the connection attempt.

Throughout this process, Cockpit acts as a signaling server, facilitating the exchange of connection metadata between the user and the Litehouse instance to establish a direct peer-to-peer WebRTC connection.
