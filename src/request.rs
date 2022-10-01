use windows::core::PWSTR;
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::System::Threading::{POWER_REQUEST_CONTEXT_SIMPLE_STRING, REASON_CONTEXT, REASON_CONTEXT_0};
use windows::Win32::System::Power::{PowerCreateRequest, PowerSetRequest, PowerRequestExecutionRequired, PowerClearRequest};

const POWER_REQUEST_CONTEXT_VERSION: u32 = 0;

pub struct PowerRequest {
    enabled: bool,
    reason: String,
    _reason_utf16: Vec<u16>, //used to keep the memory alive
    request: HANDLE,
}

impl PowerRequest {
    pub fn new(reason: String) -> Self {
        let mut v = reason.encode_utf16().collect::<Vec<u16>>();
        v.push(0);
        let context = REASON_CONTEXT {
            Version: POWER_REQUEST_CONTEXT_VERSION,
            Flags: POWER_REQUEST_CONTEXT_SIMPLE_STRING,
            Reason: REASON_CONTEXT_0 {
                SimpleReasonString: PWSTR::from_raw(&v[0] as *const u16 as *mut u16),
            }
        };
        Self {
            enabled: false,
            reason,
            _reason_utf16: v,
            request: unsafe {
                PowerCreateRequest(&context).expect("Failed to create power request")
            },
        }
    }

    pub fn reason(&self) -> &str {
        &self.reason
    }

    pub fn enter(&mut self) {
        if self.enabled {
            return;
        }
        self.enabled = true;
        unsafe {
            PowerSetRequest(self.request, PowerRequestExecutionRequired);
        }
    }

    pub fn leave(&mut self) {
        if !self.enabled {
            return;
        }
        self.enabled = false;
        unsafe {
            PowerClearRequest(self.request, PowerRequestExecutionRequired);
        }
    }
}

impl Drop for PowerRequest {
    fn drop(&mut self) {
        if self.enabled {
            self.leave();
        }
        unsafe {
            CloseHandle(self.request);
        }
    }
}
