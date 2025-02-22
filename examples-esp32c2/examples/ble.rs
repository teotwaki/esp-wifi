#![no_std]
#![no_main]

use bleps::{
    ad_structure::{
        create_advertising_data, AdStructure, BR_EDR_NOT_SUPPORTED, LE_GENERAL_DISCOVERABLE,
    },
    attribute_server::{AttributeServer, NotificationData, WorkResult},
    Ble, HciConnector,
};
use bleps_macros::gatt;
use esp_backtrace as _;
use esp_println::{logger::init_logger, println};
use esp_wifi::{ble::controller::BleConnector, initialize};
use examples_util::hal;
use hal::{
    clock::{ClockControl, CpuClock},
    peripherals::*,
    prelude::*,
    Rng, Rtc, IO,
};

#[entry]
fn main() -> ! {
    init_logger(log::LevelFilter::Info);
    esp_wifi::init_heap();

    let peripherals = Peripherals::take();

    let system = examples_util::system!(peripherals);
    let clocks = examples_util::clocks!(system);
    examples_util::rtc!(peripherals);

    let timer = examples_util::timer!(peripherals, clocks);
    initialize(timer, Rng::new(peripherals.RNG), &clocks).unwrap();

    // let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    // #[cfg(any(feature = "esp32", feature = "esp32s2", feature = "esp32s3"))]
    // let button = io.pins.gpio0.into_pull_down_input();
    // #[cfg(any(feature = "esp32c2", feature = "esp32c3"))]
    // let button = io.pins.gpio9.into_pull_down_input();
    let button = examples_util::boot_button!(peripherals);

    let mut debounce_cnt = 500;

    loop {
        let connector = BleConnector {};
        let hci = HciConnector::new(connector, esp_wifi::current_millis);
        let mut ble = Ble::new(&hci);

        println!("{:?}", ble.init());
        println!("{:?}", ble.cmd_set_le_advertising_parameters());
        println!(
            "{:?}",
            ble.cmd_set_le_advertising_data(create_advertising_data(&[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::ServiceUuids16(&[Uuid::Uuid16(0x1809)]),
                #[cfg(feature = "esp32c3")]
                AdStructure::CompleteLocalName("ESP32-C3 BLE"),
                #[cfg(feature = "esp32c2")]
                AdStructure::CompleteLocalName("ESP32-C2 BLE"),
                #[cfg(feature = "esp32")]
                AdStructure::CompleteLocalName("ESP32 BLE"),
                #[cfg(feature = "esp32s3")]
                AdStructure::CompleteLocalName("ESP32-S3 BLE"),
            ]))
        );
        println!("{:?}", ble.cmd_set_le_advertise_enable(true));

        println!("started advertising");

        let mut rf = || &b"Hello Bare-Metal BLE"[..];
        let mut wf = |offset: u16, data: &[u8]| {
            println!("RECEIVED: {} {:x?}", offset, data);
        };

        let mut wf2 = |offset: u16, data: &[u8]| {
            println!("RECEIVED: {} {:x?}", offset, data);
        };

        let mut rf3 = || &b"Hola!"[..];
        let mut wf3 = |offset: u16, data: &[u8]| {
            println!("RECEIVED: Offset {}, data {:x?}", offset, data);
        };

        gatt!([service {
            uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
            characteristics: [
                characteristic {
                    uuid: "937312e0-2354-11eb-9f10-fbc30a62cf38",
                    read: rf,
                    write: wf,
                },
                characteristic {
                    uuid: "957312e0-2354-11eb-9f10-fbc30a62cf38",
                    write: wf2,
                },
                characteristic {
                    name: "my_characteristic",
                    uuid: "987312e0-2354-11eb-9f10-fbc30a62cf38",
                    notify: true,
                    read: rf3,
                    write: wf3,
                },
            ],
        },]);

        let mut srv = AttributeServer::new(&mut ble, &mut gatt_attributes);

        loop {
            let mut notification = None;

            if button.is_low().unwrap() && debounce_cnt > 0 {
                debounce_cnt -= 1;
                if debounce_cnt == 0 {
                    if let Some(cccd) =
                        srv.get_characteristic_value(my_characteristic_notify_enable_handle)
                    {
                        // if notifications enabled
                        if cccd[0] == 1 {
                            notification = Some(NotificationData::new(
                                my_characteristic_handle,
                                &b"Notification"[..],
                            ));
                        }
                    }
                }
            };

            if button.is_high().unwrap() {
                debounce_cnt = 500;
            }

            match srv.do_work_with_notification(notification) {
                Ok(res) => {
                    if let WorkResult::GotDisconnected = res {
                        break;
                    }
                }
                Err(err) => {
                    println!("{:x?}", err);
                }
            }
        }
    }
}
