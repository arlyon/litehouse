export default {
  openapi: "3.1.0",
  info: {
    title: "Cockpit Docs",
    summary: "A webrtc signalling server for Litehouse",
    description:
      "Cockpit is the webtrc signalling server that powers litehouse. An instance of litehouse\nmay point to a cockpit broker which facilitates a direct peer-to-peer connection between\nthe web interface and the instance itself.\n\nThe signaling server is designed to require minimal resources and as such, is free to use.\nThe service does not store your instance's password or any other sensitive information either\nin memory or at rest. It is entirely stateless.\n\nTo prevent abuse, the confirmation APIs (finalize, and reject) are rate-limited per IP.\nThese two APIs\n\n# Brokering protocol\n\nThe brokering protocol in Cockpit involves several steps to establish a direct peer-to-peer WebRTC connection between a user and a Litehouse instance. Here's a user-friendly explanation of the process:\n\n1. **Instance Connection via SSE**: The Litehouse instance connects to the Cockpit server using Server-Sent Events (SSE). This connection allows the instance to receive real-time updates and offers from the server.\n2. **User Sends an Offer**: The user initiates a connection by sending an offer to the Cockpit server. This offer contains the necessary WebRTC session description to start the connection process.\n3. **Offer Forwarding**: The Cockpit server forwards the user's offer to the connected Litehouse instance. This step ensures that the instance receives the offer and can respond accordingly.\n4. **Instance Replies**: The Litehouse instance processes the received offer and replies with either an acceptance or rejection. If the instance accepts the offer, it sends back a counter-offer to complete the WebRTC connection. If the instance rejects the offer, the connection attempt is terminated.\n5. **Connection Finalization**: If the instance accepts the offer, the Cockpit server finalizes the connection by exchanging the necessary WebRTC session descriptions and ICE candidates between the user and the instance. This step establishes the direct peer-to-peer connection.\n6. **Connection Rejection**: If the instance rejects the offer, the Cockpit server informs the user that the connection attempt was unsuccessful. The user can then decide to retry or terminate the connection attempt.\n\nThroughout this process, Cockpit acts as a signaling server, facilitating the exchange of connection metadata between the user and the Litehouse instance to establish a direct peer-to-peer WebRTC connection.\n",
    version: "",
  },
  paths: {
    "/litehouse": {
      get: {
        description: "Wait for a connection from a client on the local network",
        parameters: [
          {
            in: "header",
            name: "authorization",
            required: true,
            schema: { type: "string" },
            style: "simple",
          },
        ],
        responses: {
          "200": {
            description:
              "An SSE stream that will produce an offer once it is ready",
            content: {
              "text/event-stream": {
                schema: {
                  $ref: "#/components/schemas/RTCSessionDescription",
                  externalDocs: {
                    description: "SSE",
                    url: "https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events",
                  },
                },
              },
            },
          },
        },
      },
      post: {
        description: "Finalize a pending connection",
        requestBody: {
          description:
            "A finalization sent by the server to complete a WebRTC connection",
          content: {
            "application/json": {
              schema: { $ref: "#/components/schemas/Finalize" },
            },
          },
          required: true,
        },
        responses: {
          "200": { description: "no content" },
          "400": {
            description: "There is no matching server waiting for a connection",
            content: {
              "text/plain": {
                schema: { const: "no_server" },
                example: "no_server",
              },
            },
          },
          "404": {
            description: "Client is no longer waiting for a connection",
            content: {
              "text/plain": {
                schema: { const: "client_gone" },
                example: "client_gone",
              },
            },
          },
        },
      },
      delete: {
        description: "Reject a pending connection",
        requestBody: {
          description: "The finalization was rejected",
          content: {
            "application/json": {
              schema: { $ref: "#/components/schemas/Reject" },
            },
          },
          required: true,
        },
        responses: {
          "200": {
            description: "plain text",
            content: { "text/plain; charset=utf-8": {} },
          },
        },
      },
    },
    "/litehouse/{id}": {
      get: {
        description: "Wait for a connection from a previously-paired client",
        parameters: [
          {
            in: "path",
            name: "id",
            description: "The id of the node",
            required: true,
            schema: { description: "The id of the node", type: "string" },
            style: "simple",
          },
          {
            in: "header",
            name: "authorization",
            required: true,
            schema: { type: "string" },
            style: "simple",
          },
        ],
        responses: {
          "200": {
            description:
              "An SSE stream that will produce an offer once it is ready",
            content: {
              "text/event-stream": {
                schema: {
                  $ref: "#/components/schemas/RTCSessionDescription",
                  externalDocs: {
                    description: "SSE",
                    url: "https://developer.mozilla.org/en-US/docs/Web/API/Server-sent_events",
                  },
                },
              },
            },
          },
        },
      },
    },
    "/client": {
      get: {
        description: "Get all connections available to a given client",
        parameters: [
          {
            in: "header",
            name: "authorization",
            required: true,
            schema: { type: "string" },
            style: "simple",
          },
        ],
        responses: {
          "200": {
            description: "",
            content: {
              "application/json": {
                schema: {
                  type: "array",
                  items: { $ref: "#/components/schemas/Connection" },
                },
              },
            },
          },
        },
        security: [{ test: [] }],
      },
      post: {
        description:
          "Start a new connection to an anonymous litehouse instance",
        parameters: [
          {
            in: "header",
            name: "authorization",
            required: true,
            schema: { type: "string" },
            style: "simple",
          },
        ],
        requestBody: {
          content: {
            "application/json": {
              schema: { $ref: "#/components/schemas/RTCSessionDescription" },
            },
          },
          required: true,
        },
        responses: {
          "200": {
            description: "",
            content: {
              "application/json": {
                schema: { $ref: "#/components/schemas/RTCSessionDescription" },
              },
            },
          },
          "400": {
            description: "There is no matching server waiting for a connection",
            content: {
              "text/plain": {
                schema: { const: "no_server" },
                example: "no_server",
              },
            },
          },
          "401": {
            description:
              "The connection request was rejected by the litehouse instance",
            content: {
              "text/plain": {
                schema: { const: "rejected" },
                example: "rejected",
              },
            },
          },
        },
      },
    },
    "/client/{id}": {
      post: {
        description:
          "Start a new connection to a previously-paired litehouse instance",
        parameters: [
          {
            in: "path",
            name: "id",
            description: "The id of the node",
            required: true,
            schema: { description: "The id of the node", type: "string" },
            style: "simple",
          },
          {
            in: "header",
            name: "authorization",
            required: true,
            schema: { type: "string" },
            style: "simple",
          },
        ],
        requestBody: {
          content: {
            "application/json": {
              schema: { $ref: "#/components/schemas/RTCSessionDescription" },
            },
          },
          required: true,
        },
        responses: {
          "200": {
            description: "",
            content: {
              "application/json": {
                schema: { $ref: "#/components/schemas/RTCSessionDescription" },
              },
            },
          },
          "400": {
            description: "There is no matching server waiting for a connection",
            content: {
              "text/plain": {
                schema: { const: "no_server" },
                example: "no_server",
              },
            },
          },
          "401": {
            description:
              "The connection request was rejected by the litehouse instance",
            content: {
              "text/plain": {
                schema: { const: "rejected" },
                example: "rejected",
              },
            },
          },
        },
      },
    },
  },
  components: {
    schemas: {
      Connection: {
        description: "A pending connection to the server",
        oneOf: [
          {
            type: "object",
            required: ["account", "node_id", "type"],
            properties: {
              account: {
                description: "The account that owns the node",
                type: "string",
              },
              node_id: {
                description: "The node_id of the node",
                type: "string",
              },
              type: { type: "string", enum: ["known"] },
            },
          },
          {
            type: "object",
            required: ["ip", "type"],
            properties: {
              ip: {
                description: "The ip address of the node",
                type: "string",
                format: "ip",
              },
              type: { type: "string", enum: ["unknown"] },
            },
          },
        ],
      },
      Finalize: {
        description:
          "A finalization sent by the server to complete a WebRTC connection",
        type: "object",
        required: ["id", "offer"],
        properties: {
          id: {
            description: "The ephemeral id of the connection",
            type: "integer",
            format: "uint64",
            minimum: 0.0,
          },
          offer: {
            description: "A WebRTC counter-offer to complete the connection",
            $ref: "#/components/schemas/RTCSessionDescription",
          },
        },
      },
      NodeId: {
        description: "An identifier for a particular node",
        type: "object",
        required: ["id"],
        properties: {
          id: { description: "The id of the node", type: "string" },
        },
      },
      RTCSessionDescription: {
        type: "object",
        required: ["sdp", "type"],
        properties: { sdp: { type: "string" }, type: { type: "string" } },
      },
      Reject: {
        description: "The finalization was rejected",
        type: "object",
        required: ["id"],
        properties: {
          id: {
            description: "The ephemeral id of the connection",
            type: "integer",
            format: "uint64",
            minimum: 0.0,
          },
        },
      },
    },
  },
} as const;
