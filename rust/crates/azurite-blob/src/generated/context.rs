use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};

use crate::generated::artifacts::models::{GeneratedObject, GeneratedResponse, GeneratedValue};
use crate::generated::artifacts::operation::Operation;
use crate::generated::i_request::GeneratedHttpRequest;
use crate::generated::i_response::GeneratedHttpResponse;

pub type IHandlerParameters = GeneratedObject;
pub type ContextHolder = Arc<Mutex<BTreeMap<String, Arc<Mutex<ContextState>>>>>;

#[derive(Debug, Clone, Default)]
pub struct ContextState {
    pub operation: Option<Operation>,
    pub request: Option<GeneratedHttpRequest>,
    pub dispatchPattern: Option<String>,
    pub response: Option<GeneratedHttpResponse>,
    pub handlerParameters: Option<IHandlerParameters>,
    pub handlerResponses: Option<GeneratedResponse>,
    pub contextID: Option<String>,
    pub startTime: Option<DateTime<Utc>>,
    pub extras: GeneratedObject,
}

#[derive(Debug, Clone)]
pub struct Context {
    state: Arc<Mutex<ContextState>>,
    pub path: String,
}

impl Context {
    pub fn new(context: &Context) -> Self {
        Self {
            state: Arc::clone(&context.state),
            path: context.path.clone(),
        }
    }

    pub fn new_holder() -> ContextHolder {
        Arc::new(Mutex::new(BTreeMap::new()))
    }

    pub fn from_holder(
        holder: ContextHolder,
        path: impl Into<String>,
        req: Option<GeneratedHttpRequest>,
        res: Option<GeneratedHttpResponse>,
    ) -> Self {
        let path = path.into();
        let state = {
            let mut holder_guard = holder.lock().unwrap();
            holder_guard
                .entry(path.clone())
                .or_insert_with(|| Arc::new(Mutex::new(ContextState::default())))
                .clone()
        };
        let context = Self { state, path };
        if let Some(req) = req {
            context.setRequest(Some(req));
        }
        if let Some(res) = res {
            context.setResponse(Some(res));
        }
        context
    }

    pub fn operation(&self) -> Option<Operation> {
        self.state.lock().unwrap().operation
    }

    pub fn setOperation(&self, operation: Option<Operation>) {
        self.state.lock().unwrap().operation = operation;
    }

    pub fn request(&self) -> Option<GeneratedHttpRequest> {
        self.state.lock().unwrap().request.clone()
    }

    pub fn setRequest(&self, request: Option<GeneratedHttpRequest>) {
        self.state.lock().unwrap().request = request;
    }

    pub fn dispatchPattern(&self) -> Option<String> {
        self.state.lock().unwrap().dispatchPattern.clone()
    }

    pub fn setDispatchPattern(&self, dispatchPattern: Option<String>) {
        self.state.lock().unwrap().dispatchPattern = dispatchPattern;
    }

    pub fn response(&self) -> Option<GeneratedHttpResponse> {
        self.state.lock().unwrap().response.clone()
    }

    pub fn setResponse(&self, response: Option<GeneratedHttpResponse>) {
        self.state.lock().unwrap().response = response;
    }

    pub fn handlerParameters(&self) -> Option<IHandlerParameters> {
        self.state.lock().unwrap().handlerParameters.clone()
    }

    pub fn setHandlerParameters(&self, handlerParameters: Option<IHandlerParameters>) {
        self.state.lock().unwrap().handlerParameters = handlerParameters;
    }

    pub fn handlerResponses(&self) -> Option<GeneratedResponse> {
        self.state.lock().unwrap().handlerResponses.clone()
    }

    pub fn setHandlerResponses(&self, handlerResponses: Option<GeneratedResponse>) {
        self.state.lock().unwrap().handlerResponses = handlerResponses;
    }

    pub fn contextId(&self) -> Option<String> {
        self.state.lock().unwrap().contextID.clone()
    }

    pub fn setContextId(&self, contextID: Option<String>) {
        self.state.lock().unwrap().contextID = contextID;
    }

    pub fn startTime(&self) -> Option<DateTime<Utc>> {
        self.state.lock().unwrap().startTime
    }

    pub fn setStartTime(&self, startTime: Option<DateTime<Utc>>) {
        self.state.lock().unwrap().startTime = startTime;
    }

    pub fn extras(&self) -> GeneratedObject {
        self.state.lock().unwrap().extras.clone()
    }

    pub fn insertExtra(&self, key: impl Into<String>, value: GeneratedValue) {
        self.state.lock().unwrap().extras.insert(key.into(), value);
    }
}
