use ureq::{Agent, AgentBuilder, Middleware};
// use wasi::http::{
//     outgoing_handler,
//     types::{Fields, OutgoingRequest, RequestOptions, Scheme},
// };

// Get a client that can be used to make HTTP requests
// using the wit interface. If you need to add additional
// middleware, you need to create your own `AgentBuilder`
// and add the `WasmClient` yourself. Make sure it is last.
pub fn get_client() -> AgentBuilder {
    let m = WasmClient;
    AgentBuilder::new().middleware(m)
}

struct WasmClient;

impl Middleware for WasmClient {
    fn handle(
        &self,
        request: ureq::Request,
        next: ureq::MiddlewareNext,
    ) -> Result<ureq::Response, ureq::Error> {
        // let headers = Fields::new();

        // let req = OutgoingRequest::new(headers);
        // req.set_path_with_query(Some(
        //     "/v1/forecast?latitude=51.4649&longitude=-0.1087&current=temperature_2m,wind_speed_10m",
        // ))
        // .expect("ok");
        // req.set_authority(Some("api.open-meteo.com"));
        // req.set_scheme(Some(&Scheme::Https));

        // let opts = RequestOptions::new();

        // let x = outgoing_handler::handle(req, Some(opts)).unwrap();

        todo!()
    }
}
