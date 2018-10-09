#[macro_use]
extern crate log;

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

use std::sync::atomic::AtomicPtr;

use std::os::raw::c_int;
use std::ptr;

use log::Level::Debug;

pub const STREAM: c_int = rs2_stream_RS2_STREAM_COLOR as i32; // rs2_stream is a types of data provided by RealSense device
pub const FORMAT: c_int = rs2_format_RS2_FORMAT_BGR8 as i32; // rs2_format is identifies how binary data is encoded within a frame
pub const STREAM_INDEX: i32 = 0 as i32; // Defines the stream index, used for multiple streams of the same type

pub struct RealSense {
    pipeline: *mut rs2_pipeline
}

impl RealSense {
    pub fn new(
        FPS: c_int,
        CAMERA_WIDTH: c_int,
        CAMERA_HEIGHT: c_int) -> RealSense {
        unsafe {
            // calculate API version e.g. 21400
            let api_version: std::os::raw::c_int = (
                (RS2_API_MAJOR_VERSION * 10000)
                + (RS2_API_MINOR_VERSION * 100)
                + RS2_API_PATCH_VERSION) as i32;

            let error = ptr::null_mut();

            // Create a context object. This object owns the handles to all connected realsense devices.
            // The returned object should be released with rs2_delete_context(...)
            // rs2_context* ctx = rs2_create_context(RS2_API_VERSION, &e);
            info!("Creating RS2 context");
            let ctx = rs2_create_context(api_version, error);
            check_error(error);

            /* Get a list of all the connected devices. */
            // The returned object should be released with rs2_delete_device_list(...)
            // rs2_device_list* device_list = rs2_query_devices(ctx, &e);
            info!("Querying RS2 devices");
            let device_list = rs2_query_devices(ctx, error);
            check_error(error);

            // int dev_count = rs2_get_device_count(device_list, &e);
            info!("Getting device count");
            let devices_count = rs2_get_device_count(device_list, error);
            check_error(error);

            info!("Found {:?} device(s)", devices_count);

            if devices_count == 0 {
                info!("Exiting...");
                std::process::exit(1);
            }

            // Get the first connected device
            // The returned object should be released with rs2_delete_device(...)
            // rs2_device* dev = rs2_create_device(device_list, 0, &e);
            info!("Creating RS2 device");
            let device = rs2_create_device(device_list, 0, error);
            check_error(error);

            print_device_info(&*device);

            // Create a pipeline to configure, start and stop camera streaming
            // The returned object should be released with rs2_delete_pipeline(...)
            //rs2_pipeline* pipeline =  rs2_create_pipeline(ctx, &e);
            info!("Creating pipeline");
            let pipe = rs2_create_pipeline(ctx, error);
            check_error(error);

            // Create a config instance, used to specify hardware configuration
            // The retunred object should be released with rs2_delete_config(...)
            // rs2_config* config = rs2_create_config(&e);
            info!("Creating config");
            let config = rs2_create_config(error);
            check_error(error);

            // Request a specific configuration
            info!("Enabling stream");
            rs2_config_enable_stream(
                config,
                STREAM as u32,
                STREAM_INDEX,
                CAMERA_WIDTH,
                CAMERA_HEIGHT,
                FORMAT as u32,
                FPS,
                error,
            );
            check_error(error);

            info!("Starting pipeline with config");
            let _rs2_pipeline_profile = rs2_pipeline_start_with_config(pipe, config, error);
            check_error(error);

            RealSense { pipeline: pipe}
        }
    }

    pub fn run(&mut self) -> std::sync::atomic::AtomicPtr<u8> {
        let error = ptr::null_mut();
        let mut rgb_frame_data = ptr::null_mut();

        unsafe {
            // This call waits until a new composite_frame is available
            // composite_frame holds a set of frames. It is used to prevent frame drops
            // The retunred object should be released with rs2_release_frame(...)
            let frames = rs2_pipeline_wait_for_frames(self.pipeline, 3000, error);
            check_error(error);

            // Returns the number of frames embedded within the composite frame
            let num_of_frames = rs2_embedded_frames_count(frames, error);
            check_error(error);

            // TODO extract only last frame?
            for frame_index in 0..num_of_frames {
                let frame = rs2_extract_frame(frames, frame_index, error);
                check_error(error);

                rgb_frame_data = rs2_get_frame_data(frame, error) as *mut u8;
                check_error(error);
                debug!("RGB frame arrived");

                if log_enabled!(Debug) {
                    let frame_number = rs2_get_frame_number(frame, error);
                    check_error(error);
                    debug!("Frame number {}", frame_number);

                    let frame_timestamp = rs2_get_frame_timestamp(frame, error);
                    check_error(error);
                    debug!("Frame timestamp {}", frame_timestamp);
                }

                rs2_release_frame(frame);
                debug!("Released frame");
            }

            rs2_release_frame(frames);
            debug!("Released frame wrapper");

            return AtomicPtr::new(rgb_frame_data as *mut u8);
        }
    }
}

fn check_error(e: *mut *mut rs2_error) {
    debug!("Error? {:?}\n", !e.is_null());

    if !e.is_null() {
        unsafe {
            error!(
                "rs_error was raised when calling {:?}({:?}):\n",
                rs2_get_failed_function(e as *mut rs2_error),
                rs2_get_failed_args(e as *mut rs2_error)
            );
            error!("{:?}", rs2_get_error_message(e as *mut rs2_error));
            std::process::exit(1);
        }
    }
}

fn print_device_info(device: &rs2_device) { 
    unsafe {
        let error = ptr::null_mut();

        info!(
            "Using device 0: {:?}",
            rs2_get_device_info(device, rs2_camera_info_RS2_CAMERA_INFO_NAME, error)
        );
        check_error(error);

        info!(
            "Serial number: {:?}",
            rs2_get_device_info(device, rs2_camera_info_RS2_CAMERA_INFO_SERIAL_NUMBER, error)
        );
        check_error(error);

        info!(
            "Firmware version: {:?}",
            rs2_get_device_info(
                device,
                rs2_camera_info_RS2_CAMERA_INFO_FIRMWARE_VERSION,
                error
            )
        );
        check_error(error);
    }
}
