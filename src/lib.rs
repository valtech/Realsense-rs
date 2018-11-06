#[macro_use]
extern crate log;

#[allow(dead_code)]
#[allow(non_upper_case_globals)]
#[allow(non_camel_case_types)]
mod rs_api;

use std::os::raw::c_int;
use std::ffi::CStr;

use std::ptr;
use std::slice::from_raw_parts;

use log::Level::Debug;

pub const STREAM_COLOR: c_int = rs_api::rs2_stream_RS2_STREAM_COLOR as i32; // rs2_stream is a types of data provided by RealSense device
pub const STREAM_DEPTH: c_int = rs_api::rs2_stream_RS2_STREAM_DEPTH as i32;
pub const FORMAT_BGR: c_int = rs_api::rs2_format_RS2_FORMAT_BGR8 as i32; // rs2_format is identifies how binary data is encoded within a frame
pub const FORMAT_DEPTH: c_int = rs_api::rs2_format_RS2_FORMAT_Z16 as i32;
pub const STREAM_INDEX: i32 = 0 as i32; // Defines the stream index, used for multiple streams of the same type

pub struct RealSense {
    pipeline: *mut rs_api::rs2_pipeline,
    frame_w: u32,
    frame_h: u32,
}

pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub bgr_img: Option<Box<Vec<u8>>>,
    pub depth_img: Option<Box<Vec<u8>>>,
}


impl RealSense {
    pub fn new(
        fps: c_int,
        camera_width: c_int,
        camera_height: c_int) -> Option<RealSense> {
        unsafe {
            // calculate API version e.g. 21400
            let api_version: std::os::raw::c_int = (
                (rs_api::RS2_API_MAJOR_VERSION * 10000)
                + (rs_api::RS2_API_MINOR_VERSION * 100)
                + rs_api::RS2_API_PATCH_VERSION) as i32;
            info!("Calculated API version: {}", api_version);

            let error: *mut *mut rs_api::rs2_error = ptr::null_mut();
            //let error = ptr::null_mut();

            // Create a context object. This object owns the handles to all connected realsense devices.
            // The returned object should be released with rs2_delete_context(...)
            // rs2_context* ctx = rs2_create_context(RS2_API_VERSION, &e);
            info!("Creating RS2 context");
            let ctx = rs_api::rs2_create_context(api_version, error);
            check_error(error);

            /* Get a list of all the connected devices. */
            // The returned object should be released with rs2_delete_device_list(...)
            // rs2_device_list* device_list = rs2_query_devices(ctx, &e);
            info!("Querying RS2 devices");
            let device_list = rs_api::rs2_query_devices(ctx, error);
            check_error(error);

            // int dev_count = rs2_get_device_count(device_list, &e);
            info!("Getting device count");
            let cameras_count = rs_api::rs2_get_device_count(device_list, error);
            check_error(error);

            info!("Found {:?} camera(s)", cameras_count);

            if cameras_count == 0 {
                info!("Exiting...");
                return None;
            }

            // Get the first connected device
            // The returned object should be released with rs2_delete_device(...)
            // rs2_device* dev = rs2_create_device(device_list, 0, &e);
            if let Some(device) = get_realsense_camera(device_list, cameras_count) {                    
                print_device_info(&*device);

                // Create a pipeline to configure, start and stop camera streaming
                // The returned object should be released with rs2_delete_pipeline(...)
                //rs2_pipeline* pipeline =  rs2_create_pipeline(ctx, &e);
                info!("Creating pipeline");
                let pipe = rs_api::rs2_create_pipeline(ctx, error);
                check_error(error);

                // Create a config instance, used to specify hardware configuration
                // The retunred object should be released with rs2_delete_config(...)
                // rs2_config* config = rs2_create_config(&e);
                info!("Creating config");
                let config = rs_api::rs2_create_config(error);
                check_error(error);

                // Request a specific configuration
                info!("Enabling stream");
                rs_api::rs2_config_enable_stream(
                    config,
                    STREAM_COLOR as u32,
                    STREAM_INDEX,
                    camera_width,
                    camera_height,
                    FORMAT_BGR as u32,
                    fps,
                    error,
                );
                rs_api::rs2_config_enable_stream(
                    config,
                    STREAM_DEPTH as u32,
                    STREAM_INDEX,
                    camera_width,
                    camera_height,
                    FORMAT_DEPTH as u32,
                    fps,
                    error,
                );
                check_error(error);

                info!("Starting pipeline with config");
                let _rs2_pipeline_profile = rs_api::rs2_pipeline_start_with_config(pipe, config, error);
                check_error(error);

                return Some(RealSense { pipeline: pipe, frame_w: camera_width as _, frame_h: camera_height as _});
            }

            error!("No suitable camera found");
            None
        }
    }

    pub fn run(&self) -> Frame {
        let error = ptr::null_mut();

        let mut result_frame = Frame {
            w: self.frame_w,
            h: self.frame_h,
            bgr_img: None,
            depth_img: None
        };

        unsafe {
            // This call waits until a new composite_frame is available
            // composite_frame holds a set of frames. It is used to prevent frame drops
            // The retunred object should be released with rs2_release_frame(...)
            let frames = rs_api::rs2_pipeline_wait_for_frames(self.pipeline, 3000, error);
            check_error(error);

            // Returns the number of frames embedded within the composite frame
            let num_of_frames = rs_api::rs2_embedded_frames_count(frames, error);
            check_error(error);

            for i in 0..num_of_frames {
                let frame = rs_api::rs2_extract_frame(frames, i, error);
                check_error(error);

                let rgb_frame_data = rs_api::rs2_get_frame_data(frame, error) as *mut u8;
                check_error(error);
                debug!("RGB frame arrived");

                if 1 == rs_api::rs2_is_frame_extendable_to(frame, rs_api::rs2_extension_RS2_EXTENSION_DEPTH_FRAME, error) {
                    let mut bytebuf = vec![0; (3*self.frame_w*self.frame_h) as usize];
                    let mut ptr: *const u16 = rgb_frame_data as *mut u16;
                    let end = ptr.wrapping_offset((self.frame_w*self.frame_h) as isize);
                    let mut i = 0;
                    while ptr != end {
                        let pixel_data = (*ptr as f32/8.0) as u8;

                        bytebuf[i] = pixel_data;
                        bytebuf[i+1] = pixel_data;
                        bytebuf[i+2] = pixel_data;
                        ptr = ptr.wrapping_offset(1);

                        i += 3;
                    }
                    result_frame.depth_img = Some(Box::new(bytebuf)); 
                    
                } else if 1 == rs_api::rs2_is_frame_extendable_to(frame, rs_api::rs2_extension_RS2_EXTENSION_VIDEO_FRAME, error) {
                    result_frame.bgr_img = Some(Box::new(from_raw_parts(
                            rgb_frame_data,
                            (self.frame_w * self.frame_h * 3) as usize,
                        ).to_vec()));
                }
            
                if log_enabled!(Debug) {
                    let frame_number = rs_api::rs2_get_frame_number(frame, error);
                    check_error(error);
                    debug!("Frame number {}", frame_number);

                    let frame_timestamp = rs_api::rs2_get_frame_timestamp(frame, error);
                    check_error(error);
                    debug!("Frame timestamp {}", frame_timestamp);
                }

                rs_api::rs2_release_frame(frame);
                debug!("Released frame");
            }

            rs_api::rs2_release_frame(frames);
            debug!("Released frame wrapper");

            return result_frame;
        }
    }
}

fn get_realsense_camera(device_list: *mut rs_api::rs2_device_list, cameras_count: i32) -> Option<*mut rs_api::rs2_device> {
    let error = ptr::null_mut();
    (0..cameras_count)
        .map(|i| unsafe {
            let device = rs_api::rs2_create_device(device_list, i, error);
            check_error(error);
            device
        })
        .fold(None, |acc, d| {
            let name = fetch_camera_name(unsafe {&*d});
            info!("Found device with name: {}", name);
            if name.contains("Intel RealSense") {
                info!("Selecting device: {}", name);
                Some(d)
            } else {
                acc
            }
        })
}

fn fetch_camera_name(device: &rs_api::rs2_device) -> &str {
    let error = ptr::null_mut();
    let c_str: &CStr = unsafe { CStr::from_ptr(rs_api::rs2_get_device_info(device, rs_api::rs2_camera_info_RS2_CAMERA_INFO_NAME, error)) };
    check_error(error);

    c_str.to_str().expect("Could not fetch camera info name")
}

fn check_error(e: *mut *mut rs_api::rs2_error) {
    if !e.is_null() {
        unsafe {
            error!(
                "rs_error was raised when calling {:?}({:?}):\n",
                rs_api::rs2_get_failed_function(e as *mut rs_api::rs2_error),
                rs_api::rs2_get_failed_args(e as *mut rs_api::rs2_error)
            );
            error!("{:?}", rs_api::rs2_get_error_message(e as *mut rs_api::rs2_error));
            std::process::exit(1);
        }
    }
}

fn print_device_info(device: &rs_api::rs2_device) { 
    info!("Using device 0: {}", fetch_camera_name(device));

    let error = ptr::null_mut();
    {
        let c_str: &CStr = unsafe { CStr::from_ptr(
            rs_api::rs2_get_device_info(
                device,
                rs_api::rs2_camera_info_RS2_CAMERA_INFO_SERIAL_NUMBER,
                error))
        };
        let str_slice: &str = c_str.to_str().expect("Could not fetch camera serial number");
        info!("Serial number: {}", str_slice);
    }
    check_error(error);

    {
        let c_str: &CStr = unsafe { CStr::from_ptr(
            rs_api::rs2_get_device_info(
                device,
                rs_api::rs2_camera_info_RS2_CAMERA_INFO_FIRMWARE_VERSION,
                error
                ))
        };

        let str_slice: &str = c_str.to_str().expect("Could not fetch camera firmware version");
        info!("Firmware version: {}", str_slice);
    }
    check_error(error);
}
