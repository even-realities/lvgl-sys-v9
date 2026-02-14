use lvgl_sys::*;
use std::sync::atomic::{AtomicPtr, Ordering};

const HOR_RES: i32 = 480;
const VER_RES: i32 = 320;
const BUF_SIZE: usize = (HOR_RES * VER_RES * 2) as usize;

// Store framebuffer pointer for flush callback
static FRAMEBUFFER: AtomicPtr<u8> = AtomicPtr::new(std::ptr::null_mut());

unsafe extern "C" fn flush_cb(
    disp: *mut lv_display_t,
    area: *const lv_area_t,
    px_map: *mut u8,
) {
    let fb = FRAMEBUFFER.load(Ordering::Relaxed);
    let area = &*area;
    let width = area.x2 - area.x1 + 1;
    let height = area.y2 - area.y1 + 1;

    // Copy to framebuffer (RGB565 = 2 bytes per pixel)
    for y in 0..height {
        for x in 0..width {
            let src_idx = ((y * width + x) * 2) as usize;
            let dst_x = area.x1 + x;
            let dst_y = area.y1 + y;
            let dst_idx = ((dst_y * HOR_RES + dst_x) * 2) as usize;

            *fb.add(dst_idx) = *px_map.add(src_idx);
            *fb.add(dst_idx + 1) = *px_map.add(src_idx + 1);
        }
    }

    lv_display_flush_ready(disp);
}

fn main() {
    // Allocate buffers on heap
    let mut framebuffer = vec![0u8; BUF_SIZE].into_boxed_slice();
    let mut draw_buf = vec![0u8; BUF_SIZE].into_boxed_slice();

    // Store framebuffer pointer for callback
    FRAMEBUFFER.store(framebuffer.as_mut_ptr(), Ordering::Relaxed);

    unsafe {
        // Initialize LVGL
        lv_init();

        // Create display
        let disp = lv_display_create(HOR_RES, VER_RES);

        // Set up draw buffer
        lv_display_set_buffers(
            disp,
            draw_buf.as_mut_ptr() as *mut _,
            std::ptr::null_mut(),
            draw_buf.len() as u32,
            lv_display_render_mode_t_LV_DISPLAY_RENDER_MODE_PARTIAL,
        );

        // Set flush callback
        lv_display_set_flush_cb(disp, Some(flush_cb));

        // Get active screen
        let screen = lv_screen_active();

        // Create an arc configured as a full circle
        let arc = lv_arc_create(screen);
        lv_obj_set_size(arc, 150, 150);
        lv_obj_center(arc);

        // Make it a full circle (background arc from 0 to 360)
        lv_arc_set_bg_angles(arc, 0, 360);
        lv_arc_set_angles(arc, 0, 270);  // Foreground arc shows 270 degrees

        // Remove the knob
        lv_obj_remove_style(arc, std::ptr::null_mut(), LV_PART_KNOB);

        // Run for a few frames to render
        for _ in 0..10 {
            lv_tick_inc(16);
            lv_timer_handler();
        }

        println!("Circle rendered to framebuffer!");
    }

    // Save as PPM (RGB565 -> RGB888)
    let mut ppm = Vec::with_capacity(HOR_RES as usize * VER_RES as usize * 3 + 50);
    ppm.extend_from_slice(format!("P6\n{} {}\n255\n", HOR_RES, VER_RES).as_bytes());

    for y in 0..VER_RES {
        for x in 0..HOR_RES {
            let idx = ((y * HOR_RES + x) * 2) as usize;
            let lo = framebuffer[idx];
            let hi = framebuffer[idx + 1];
            let rgb565 = (hi as u16) << 8 | lo as u16;

                // RGB565: RRRRRGGG GGGBBBBB
                let r = ((rgb565 >> 11) & 0x1F) as u8;
                let g = ((rgb565 >> 5) & 0x3F) as u8;
                let b = (rgb565 & 0x1F) as u8;

                // Scale to 8-bit
            ppm.push((r << 3) | (r >> 2));
            ppm.push((g << 2) | (g >> 4));
            ppm.push((b << 3) | (b >> 2));
        }
    }

    std::fs::write("circle.ppm", &ppm).unwrap();
    println!("Saved to circle.ppm");
}
