use cocoa::appkit::NSApplicationActivationPolicy::NSApplicationActivationPolicyRegular;
use cocoa::appkit::NSBackingStoreType::NSBackingStoreBuffered;
use cocoa::appkit::NSEventType::NSMouseMoved;
use cocoa::appkit::{
    NSApp, NSApplication, NSColor, NSEvent, NSEventMask, NSImage, NSView, NSWindow,
    NSWindowStyleMask,
};
use cocoa::base::{id, nil, NO, YES};
use cocoa::delegate;
use cocoa::foundation::{NSDefaultRunLoopMode, NSInteger, NSPoint, NSRect, NSSize, NSString};
use libc;
use objc::rc::autoreleasepool;
use objc::runtime::{Object, Sel};
use objc::*;
use std::ffi::CStr;
use std::ops::{Deref, DerefMut};

static mut RUNNING: bool = true;

struct Window {
    origin: (f64, f64),
    size: (f64, f64),
}

impl Window {
    const fn new() -> Self {
        Self {
            origin: (600.0, 600.0),
            size: (500.0, 500.0),
        }
    }

    fn origin(&self) -> NSPoint {
        NSPoint::new(self.origin.0, self.origin.1)
    }

    fn size(&self) -> NSSize {
        NSSize::new(self.size.0, self.size.1)
    }
}

static mut WINDOW: Window = Window::new();

extern "C" fn on_window_will_close(_this: &Object, _cmd: Sel, _notification: id) {
    println!("Window will close!");
    unsafe { RUNNING = false };
}

extern "C" fn on_window_did_resize(_this: &Object, _cmd: Sel, notification: id) {
    let frame = unsafe {
        let object: id = msg_send![notification, object];
        NSWindow::frame(object)
    };

    unsafe {
        WINDOW.size = (frame.size.width, frame.size.height);
        println!("Window resized: {:?}", WINDOW.size);
    };
}

extern "C" fn on_window_did_move(_this: &Object, _cmd: Sel, notification: id) {
    let frame = unsafe {
        let object: id = msg_send![notification, object];
        NSWindow::frame(object)
    };

    unsafe {
        WINDOW.origin = (frame.origin.x, frame.origin.y);
        println!("Window moved: {:?}", WINDOW.origin);
    };
}

fn main() {
    unsafe { main_() };
}

/// Basically the same as Box<[T]>
struct BoxedSlice<T> {
    ptr: *mut T,
    size: usize,
}

impl<T> BoxedSlice<T> {}

impl<T> Deref for BoxedSlice<T> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        unsafe { std::slice::from_raw_parts(self.ptr, self.size) }
    }
}
impl<T> DerefMut for BoxedSlice<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { std::slice::from_raw_parts_mut(self.ptr, self.size) }
    }
}

struct Buffer {
    allocation: BoxedSlice<u8>,
    width: usize,  // Size in pixels
    height: usize, // Size in pixels
}

impl Buffer {
    fn clear(&mut self) {
        self.allocation.fill(0);
    }

    fn draw_square(&mut self, x: isize, y: isize, width: usize, height: usize) {
        for i in x..x + (width as isize) {
            for j in y..y + (height as isize) {
                self.set_pixel(i, j);
            }
        }
    }

    fn set_pixel(&mut self, x: isize, y: isize) {
        if x < 0 || x >= (self.width as isize) || y < 0 || y >= (self.height as isize) {
            return;
        }
        let pixel_offset = x as usize + y as usize * self.width;

        // TODO: Not hardcode this to white
        (&mut self.allocation)[4 * pixel_offset + 0] = 0xFF;
        (&mut self.allocation)[4 * pixel_offset + 1] = 0xFF;
        (&mut self.allocation)[4 * pixel_offset + 2] = 0xFF;
        (&mut self.allocation)[4 * pixel_offset + 3] = 0xFF;
    }
}

unsafe fn main_() {
    let app = NSApp();
    app.setActivationPolicy_(NSApplicationActivationPolicyRegular);
    app.activateIgnoringOtherApps_(YES);

    let window_rect = NSRect::new(WINDOW.origin(), WINDOW.size());

    let window = NSWindow::alloc(nil).initWithContentRect_styleMask_backing_defer_(
        window_rect,
        NSWindowStyleMask::NSMiniaturizableWindowMask
            | NSWindowStyleMask::NSClosableWindowMask
            | NSWindowStyleMask::NSResizableWindowMask
            | NSWindowStyleMask::NSTitledWindowMask,
        NSBackingStoreBuffered,
        NO,
    );

    let title = NSString::alloc(nil).init_str("My window");
    window.setTitle_(title);

    window.makeKeyAndOrderFront_(nil);

    window.setDelegate_(delegate!("MyWindowDelegate", {
        window: id = window,
        (windowWillClose:) => on_window_will_close as extern fn(&Object, Sel, id),
        (windowDidResize:) => on_window_did_resize as extern fn(&Object, Sel, id),
        (windowDidMove:) => on_window_did_move as extern fn(&Object, Sel, id)
    }));

    // BUFFER
    let bytes_per_pixel: usize = 4;
    let window_width = WINDOW.size().width as usize;
    let window_height = WINDOW.size().height as usize;
    let row_size = window_width as usize * bytes_per_pixel;
    let buffer_size = row_size * window_height as usize;

    // let layout = Layout::array::<u8>(buffer_size * 10).expect("Couldn't create layout for buffer");
    let buffer = libc::malloc(buffer_size) as *mut u8;
    buffer.write_bytes(200, 30 * row_size);
    // let buffer_pointer: *mut *mut u8 = &mut buffer;

    let color = NSColor::colorWithDeviceRed_green_blue_alpha_(nil, 1.0, 1.0, 1.0, 1.0);
    let color_space_name: id = msg_send![color, colorSpaceName];

    let str = CStr::from_ptr(color_space_name.UTF8String());
    println!("ColorSpaceName: {}", str.to_str().unwrap());

    let w: NSInteger = WINDOW.size.0 as NSInteger;
    let h: NSInteger = 500;
    let bps: NSInteger = 8;
    let bpp: NSInteger = 32;

    println!("got here");
    let mut actual_buffer = Buffer {
        allocation: BoxedSlice {
            ptr: buffer,
            size: buffer_size,
        },
        width: w as usize,
        height: h as usize,
    };

    let mut mouse = NSPoint::new(0.0, 0.0);

    while RUNNING {
        actual_buffer.clear();
        actual_buffer.draw_square(mouse.x as isize, mouse.y as isize, 100, 150);

        autoreleasepool(|| {
            let rep: id = msg_send![class!(NSBitmapImageRep), alloc];
            let image_rep: id = msg_send![rep,
            initWithBitmapDataPlanes: &buffer
                          pixelsWide: w
                          pixelsHigh: h
                       bitsPerSample: bps
                     samplesPerPixel: bytes_per_pixel
                            hasAlpha: YES
                            isPlanar: NO
                      colorSpaceName: color_space_name
                         bytesPerRow: row_size
                        bitsPerPixel: bpp];
            let image_size = NSSize::new(w as f64, h as f64);
            let image = NSImage::alloc(nil).initWithSize_(image_size);
            image.addRepresentation_(image_rep);
            let layer: id = window.contentView().layer();
            let _: id = msg_send![layer, setContents: image];

            // Both these and the autoreleasepool are necessary apparently
            let _: () = msg_send![image, release];
            let _: () = msg_send![image_rep, release];
        });

        loop {
            let event = app.nextEventMatchingMask_untilDate_inMode_dequeue_(
                NSEventMask::NSAnyEventMask.bits(),
                nil,
                NSDefaultRunLoopMode,
                YES,
            );

            if event == nil {
                break;
            }

            match NSEvent::eventType(event) {
                NSMouseMoved => {
                    let mouse_global_location = NSEvent::mouseLocation(event);
                    mouse = to_mouse_location_relative(mouse_global_location);
                    println!("MouseMoved: {:?}", (mouse.x, mouse.y));
                }
                event_type => {
                    println!("{:?}", event_type);
                }
            }
            app.sendEvent_(event)
        }
    }

    println!("Quitting, bye!");
}

unsafe fn to_mouse_location_relative(mouse: NSPoint) -> NSPoint {
    const MAC_HEIGHT: f64 = 1280.0;
    NSPoint::new(mouse.x - WINDOW.origin().x, MAC_HEIGHT - mouse.y)
}
