use anyhow::Result;
use std::{env, mem, os::raw::c_void, ptr};

use windows::{
    core::{HSTRING, PCWSTR, PWSTR},
    Wdk::System::Threading::{NtSetInformationThread, ThreadHideFromDebugger},
    Win32::{
        Foundation::GetLastError,
        System::{
            Memory::{GetProcessHeap, HeapAlloc, HEAP_ZERO_MEMORY},
            SystemServices::{
                PROCESS_MITIGATION_BINARY_SIGNATURE_POLICY, PROCESS_MITIGATION_DYNAMIC_CODE_POLICY,
                SE_SIGNING_LEVEL_DYNAMIC_CODEGEN, SE_SIGNING_LEVEL_MICROSOFT,
            },
            Threading::{
                CreateProcessW, DeleteProcThreadAttributeList, GetCurrentThread,
                InitializeProcThreadAttributeList, ProcessDynamicCodePolicy,
                ProcessSignaturePolicy, SetProcessMitigationPolicy, UpdateProcThreadAttribute,
                EXTENDED_STARTUPINFO_PRESENT, LPPROC_THREAD_ATTRIBUTE_LIST, PROCESS_INFORMATION,
                PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY, STARTUPINFOEXW, STARTUPINFOW_FLAGS,
            },
        },
    },
};

const PROCESS_CREATION_MITIGATION_POLICY_BLOCK_NON_MICROSOFT_BINARIES_ALWAYS_ON: u64 =
    0x0000_0001_u64 << 44;

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

fn get_filename() -> Result<String> {
    match env::current_exe() {
        Ok(path) => {
            if let Some(name) = path.file_name() {
                if let Some(name) = name.to_str() {
                    Ok(name.to_owned())
                } else {
                    error!("Failed to get current exe path: {:?}", path);
                    Err(anyhow::anyhow!("Failed to get current exe path"))
                }
            } else {
                error!("Failed to get current exe path: {:?}", path);
                Err(anyhow::anyhow!("Failed to get current exe path"))
            }
        }
        Err(err) => {
            error!("Failed to get current exe path: {:?}", err);
            Err(anyhow::anyhow!("Failed to get current exe path"))
        }
    }
}

unsafe fn get_dll_attributes() -> Result<LPPROC_THREAD_ATTRIBUTE_LIST> {
    let mut attribute_size = usize::default();

    // The first call returns an error, this is intentional
    let _ = InitializeProcThreadAttributeList(
        LPPROC_THREAD_ATTRIBUTE_LIST(ptr::null_mut()),
        1,
        0,
        &mut attribute_size,
    );

    let attributes = LPPROC_THREAD_ATTRIBUTE_LIST(HeapAlloc(
        GetProcessHeap()?,
        HEAP_ZERO_MEMORY,
        attribute_size,
    ));

    match InitializeProcThreadAttributeList(attributes, 1, 0, &mut attribute_size) {
        Ok(()) => {
            info!("Initialized attribute list");
        }
        Err(err) => {
            error!("Failed to initialize attribute list: {:?}", err);
            return Err(anyhow::anyhow!("Failed to initialize attribute list"));
        }
    }

    let policy = PROCESS_CREATION_MITIGATION_POLICY_BLOCK_NON_MICROSOFT_BINARIES_ALWAYS_ON;

    match UpdateProcThreadAttribute(
        attributes,
        0,
        PROC_THREAD_ATTRIBUTE_MITIGATION_POLICY as usize,
        Some(ptr::from_ref(&policy).cast::<c_void>()),
        std::mem::size_of::<u64>(),
        None,
        None,
    ) {
        Ok(()) => {
            info!("Updated attribute list");
        }
        Err(err) => {
            error!("Failed to update attribute list: {:?}", err);
            return Err(anyhow::anyhow!("Failed to update attribute list"));
        }
    }
    Ok(attributes)
}

/// Launches a new instance of an application with the specified command.
///
/// # Arguments
///
/// * `command` - An optional command to launch the new instance with. If `None`, the function will log an error.
///
/// # Returns
///
/// A `Result` indicating the success or failure of launching the new instance.
///
/// # Errors
///
/// This function will return an error in the following situations:
///
/// * If retrieving the filename of the current executable fails.
/// * If any system calls made within the function fail,
/// such as those involved in setting up the process startup information or launching the new instance itself.
pub fn launch_new_instance(command: Option<&str>) -> Result<()> {
    let mut app_name = get_filename()?;

    match command {
        Some(command) => {
            info!("Command: {:?}", command);
            app_name = format!("\"{app_name}\" {command}");
        }
        None => {
            error!("Command not provided");
        }
    }
    info!("App name: {:?}", app_name);
    let app_name_wide_ptr = HSTRING::from(app_name.clone())
        .as_wide()
        .as_ptr()
        .cast_mut();

    unsafe {
        let mut startup_info = STARTUPINFOEXW::default();
        startup_info.StartupInfo.cb = u32::try_from(std::mem::size_of::<STARTUPINFOEXW>())?;
        startup_info.StartupInfo.dwFlags = STARTUPINFOW_FLAGS(EXTENDED_STARTUPINFO_PRESENT.0);

        let attributes = get_dll_attributes()?;
        startup_info.lpAttributeList = attributes;

        let mut process_info = PROCESS_INFORMATION::default();

        let status = match CreateProcessW(
            PCWSTR::null(),
            PWSTR::from_raw(app_name_wide_ptr),
            None,
            None,
            false,
            EXTENDED_STARTUPINFO_PRESENT,
            None,
            None,
            &startup_info.StartupInfo,
            &mut process_info,
        ) {
            Ok(()) => {
                info!("Created process: {:?}", app_name);
                Ok(())
            }
            Err(err) => {
                error!("Failed to create process: {} - | {:?}", err, GetLastError());
                Err(anyhow::anyhow!("Failed to create process"))
            }
        };
        DeleteProcThreadAttributeList(attributes);
        status
    }
}

pub fn apply_mitigations() {
    hide_current_thread_from_debuggers();
    prevent_third_party_dll_loading();
    enable_arbitrary_code_guard();
}
