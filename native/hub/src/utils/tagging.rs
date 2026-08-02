use rinf::{DartSignal, RustSignal};

use crate::signals::server::{
    CreateTaggingRequest, CreateTaggingResponse, PutTaggingRequest, PutTaggingResponse,
};
use crate::utils::logger;
use tawai_core::utils::tagging::{dii, oc};

pub async fn handle_tagging() {
    let create_receive = CreateTaggingRequest::get_dart_signal_receiver();
    let put_receive = PutTaggingRequest::get_dart_signal_receiver();

    loop {
        tokio::select! {
            Some(signal_pack) = create_receive.recv() => {
                let msg = signal_pack.message;
                let output = oc(&msg.name, &msg.description)
                    .unwrap_or_else(|e| {
                        logger::error(&format!("Create tagging error: {:?}", e));
                        String::new()
                    });
                CreateTaggingResponse { id: msg.id, success: output }.send_signal_to_dart();
            }
            Some(signal_pack) = put_receive.recv() => {
                let msg = signal_pack.message;
                let output = dii(&msg.name, &msg.tag)
                    .unwrap_or_else(|e| {
                        logger::error(&format!("Put tagging error: {:?}", e));
                        String::new()
                    });
                PutTaggingResponse { id: msg.id, success: output }.send_signal_to_dart();
            }
        }
    }
}
