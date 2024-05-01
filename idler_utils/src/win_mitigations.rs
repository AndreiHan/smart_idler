use std::{mem, os::raw::c_void, ptr};

use windows::{
    Wdk::System::Threading::{NtSetInformationThread, ThreadHideFromDebugger},
    Win32::System::{
        SystemServices::{
            PROCESS_MITIGATION_BINARY_SIGNATURE_POLICY, PROCESS_MITIGATION_DYNAMIC_CODE_POLICY,
            SE_SIGNING_LEVEL_DYNAMIC_CODEGEN, SE_SIGNING_LEVEL_MICROSOFT,
        },
        Threading::{
            GetCurrentThread, ProcessDynamicCodePolicy, ProcessSignaturePolicy,
            SetProcessMitigationPolicy,
        },
    },
};

pub fn hide_current_thread_from_debuggers() {
    unsafe {
        let status =
            NtSetInformationThread(GetCurrentThread(), ThreadHideFromDebugger, ptr::null(), 0);
        info!("Set anti debug status: {:?}", status);
    }
}

fn prevent_third_party_dll_loading() {
    let mut policy = PROCESS_MITIGATION_BINARY_SIGNATURE_POLICY::default();
    policy.Anonymous.Flags = SE_SIGNING_LEVEL_MICROSOFT;
    policy.Anonymous.Anonymous._bitfield = 1;

    unsafe {
        let status = SetProcessMitigationPolicy(
            ProcessSignaturePolicy,
            std::ptr::addr_of!(policy).cast::<c_void>(),
            mem::size_of_val(&policy),
        );
        info!("Set process mitigation policy status: {:?}", status);
    }
}

fn enable_arbitrary_code_guard() {
    let mut policy = PROCESS_MITIGATION_DYNAMIC_CODE_POLICY::default();
    policy.Anonymous.Flags = SE_SIGNING_LEVEL_DYNAMIC_CODEGEN;
    policy.Anonymous.Anonymous._bitfield = 1;

    unsafe {
        let status = SetProcessMitigationPolicy(
            ProcessDynamicCodePolicy,
            std::ptr::addr_of!(policy).cast::<c_void>(),
            mem::size_of_val(&policy),
        );
        info!("Set process mitigation policy status: {:?}", status);
    }
}

pub fn apply_mitigations() {
    hide_current_thread_from_debuggers();
    prevent_third_party_dll_loading();
    enable_arbitrary_code_guard();
}
