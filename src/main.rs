use winapi::um::bluetoothapis::*;
use winapi::ctypes::c_void;
use winapi::shared::bthdef::*;

use std::mem::size_of;
use std::ptr::{null_mut};
use std::mem::{zeroed};
use widestring::{U16CStr,U16CString};
use anyhow::{Result, anyhow};
use std::io;
use std::io::prelude::*;

struct BluetoothDevice {
    name: String,
    device_info: BLUETOOTH_DEVICE_INFO
}

fn u16_array_to_string(u16_bytes: &[u16]) -> Result<String> {
    Ok(U16CStr::from_slice_with_nul(u16_bytes)?.to_string()?)
}

fn get_pro_controller(show_known: bool) -> Result<BluetoothDevice> {
    let search_params = BLUETOOTH_DEVICE_SEARCH_PARAMS {
        dwSize: size_of::<BLUETOOTH_DEVICE_SEARCH_PARAMS>() as u32,
        fReturnAuthenticated: if show_known {1} else {0},
        fReturnRemembered : if show_known {1} else {0},
        fReturnConnected : if show_known {1} else {0},
        fReturnUnknown: if show_known {0} else {1},
        fIssueInquiry : 1,
        cTimeoutMultiplier: 2,
        hRadio: null_mut()
    };

    unsafe {
        let mut device_info = BLUETOOTH_DEVICE_INFO {
            dwSize: size_of::<BLUETOOTH_DEVICE_INFO>() as u32,
            Address: 0,
            ulClassofDevice: 0,
            fConnected: 0,
            fRemembered: 0,
            fAuthenticated: 0,
            stLastSeen: zeroed(),
            stLastUsed: zeroed(),
            szName: zeroed()
        };

        let found_device = BluetoothFindFirstDevice(&search_params, &mut device_info);

        if found_device == null_mut() {
            return Err(anyhow!("No devices found"))
        }

        let u16_str = u16_array_to_string(&device_info.szName)?;

        let device = BluetoothDevice {name: u16_str, device_info};

        if device.name == "Pro Controller" {
            return Ok(device)
        }
        
        while BluetoothFindNextDevice(found_device, &mut device_info) == 1 {
            let device = BluetoothDevice {name: u16_array_to_string(&device_info.szName)?, device_info};
            if device.name == "Pro Controller" {
                return Ok(device)
            }    
        }
    }

    return Err(anyhow!("No Pro Controller found"))
}

unsafe extern "system" fn bluetooth_registration_callback(_: *mut c_void, callback_params: *mut BLUETOOTH_AUTHENTICATION_CALLBACK_PARAMS) -> i32 {
    let mut auth_res: BLUETOOTH_AUTHENTICATE_RESPONSE = std::mem::zeroed();

    auth_res.authMethod = (&*callback_params).authenticationMethod;
    auth_res.bthAddressRemote = (&*callback_params).deviceInfo.Address;
    auth_res.negativeResponse = 0;
    *auth_res.u.numericCompInfo_mut() = BLUETOOTH_NUMERIC_COMPARISON_INFO  { NumericValue: *((&*callback_params).u.Numeric_Value()) };

    println!("About to respond");

    println!("Sending auth response {}", BluetoothSendAuthenticationResponseEx(null_mut(), &mut auth_res));

    println!("Responded");

    // You can return anything here, honestly just check the docs idk why
    1
}

fn main() -> Result<()> {
    unsafe { 
        // Remove pro controller if already paired
        match get_pro_controller(true) {
            Ok(device) => {
                BluetoothRemoveDevice(&device.device_info.Address);
                println!("Removed controller");
            },
            Err(err) => println!("Error finding controller {}", err)
        }

        println!("Sync controller then smash that return key");
        let _ = io::stdin().read(&mut [0u8]).unwrap();
        println!("Lookin for pros");

        let mut device = get_pro_controller(false)?;

        println!("Found it. Authenticating");

        let mut callback_handle: HBLUETOOTH_AUTHENTICATION_REGISTRATION = null_mut();

        // Really we should clean this up but it's pointless with code this short-running
        println!("Registering callback {}", BluetoothRegisterForAuthenticationEx(&mut device.device_info, &mut callback_handle as *mut HBLUETOOTH_AUTHENTICATION_REGISTRATION, Some(bluetooth_registration_callback), null_mut()));

        println!("Authing {}", BluetoothAuthenticateDevice(null_mut(), null_mut(), &mut device.device_info, U16CString::from_str("0000").unwrap().into_raw(), 4));

        BluetoothSetServiceState(null_mut(), &mut device.device_info, &mut HumanInterfaceDeviceServiceClass_UUID, BLUETOOTH_SERVICE_ENABLE);
    }

    Ok(())

}