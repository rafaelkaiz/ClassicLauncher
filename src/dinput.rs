//! Game controllers through DirectInput 8, the way openOMSI reads them (ported from its `omsi-input/src/dinput.rs`): the same list of
//! devices with the same names, and the eight axes in the slots `gamectrler.cfg` numbers (X, Y, Z, X rotation, Y rotation,
//! Z rotation and the two sliders). Windows' older joystick interface lists fewer devices and fewer axes, so the page uses this
//! one. Everything runs on the thread that draws the window.

use std::cell::RefCell;
use windows::Win32::Devices::HumanInterfaceDevice::*;
use windows::Win32::Foundation::{HINSTANCE, HWND};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{GetDesktopWindow, GetForegroundWindow};
use windows::core::{GUID, Interface};

const RANGE: i32 = 10_000;

/// What a device gives: the axes in their slots, the four POV hats and 128 buttons (the layout of the data format below).
#[repr(C)]
#[derive(Clone, Copy)]
struct RawState {
    axes: [i32; 8],
    pov: [u32; 4],
    buttons: [u8; 128],
}

struct Open {
    name: String,
    guid: GUID,
    dev: IDirectInputDevice8W,
    has_axis: [bool; 8],
    ranges: [(i32, i32); 8],
}

#[derive(Default)]
struct Pads {
    di: Option<IDirectInput8W>,
    /// The devices found by the last look, and the ones opened so far.
    found: Vec<(GUID, String)>,
    open: Vec<Open>,
}

thread_local! {
    static PADS: RefCell<Pads> = RefCell::new(Pads::default());
}

fn is_controller_device(dev_type: u32, usage_page: u16, usage: u16) -> bool {
    let primary = dev_type & 0xFF;
    if matches!(primary, DI8DEVTYPE_JOYSTICK | DI8DEVTYPE_GAMEPAD | DI8DEVTYPE_DRIVING | DI8DEVTYPE_FLIGHT | DI8DEVTYPE_1STPERSON | DI8DEVTYPE_DEVICECTRL | DI8DEVTYPE_SUPPLEMENTAL) {
        return true;
    }
    if primary == DI8DEVTYPE_DEVICE {
        return match usage_page {
            0x01 => matches!(usage, 0x04 | 0x05 | 0x08),
            0x02 => true,
            0x04 | 0x05 => true,
            _ => false,
        };
    }
    false
}

unsafe extern "system" fn collect(inst: *mut DIDEVICEINSTANCEW, out: *mut core::ffi::c_void) -> windows::core::BOOL {
    let v = unsafe { &mut *(out as *mut Vec<(GUID, String)>) };
    let inst = unsafe { &*inst };
    if is_controller_device(inst.dwDevType, inst.wUsagePage, inst.wUsage) {
        let end = inst.tszProductName.iter().position(|c| *c == 0).unwrap_or(inst.tszProductName.len());
        let name = String::from_utf16_lossy(&inst.tszProductName[..end]).trim().to_string();
        if !v.iter().any(|(g, _)| *g == inst.guidInstance) {
            v.push((inst.guidInstance, name));
        }
    }
    windows::core::BOOL(DIENUM_CONTINUE as i32)
}

#[derive(Clone, Copy)]
struct InputObject {
    guid: GUID,
    ty: u32,
    flags: u32,
}

unsafe extern "system" fn collect_object(inst: *mut DIDEVICEOBJECTINSTANCEW, out: *mut core::ffi::c_void) -> windows::core::BOOL {
    let objects = unsafe { &mut *(out as *mut Vec<InputObject>) };
    let inst = unsafe { &*inst };
    objects.push(InputObject { guid: inst.guidType, ty: inst.dwType, flags: inst.dwFlags });
    windows::core::BOOL(DIENUM_CONTINUE as i32)
}

fn axis_slot(guid: GUID, has_axis: &[bool; 8]) -> Option<usize> {
    if guid == GUID_XAxis {
        Some(0)
    } else if guid == GUID_YAxis {
        Some(1)
    } else if guid == GUID_ZAxis {
        Some(2)
    } else if guid == GUID_RxAxis {
        Some(3)
    } else if guid == GUID_RyAxis {
        Some(4)
    } else if guid == GUID_RzAxis {
        Some(5)
    } else if guid == GUID_Slider {
        (6..8).find(|k| !has_axis[*k])
    } else {
        None
    }
}

fn format_objects(objects: &[InputObject]) -> (Vec<DIOBJECTDATAFORMAT>, [bool; 8]) {
    let mut objs = Vec::new();
    let mut has_axis = [false; 8];
    let mut pov = 0;
    let mut buttons = [false; 128];
    for object in objects {
        let (offset, flags) = if object.ty & DIDFT_AXIS != 0 {
            let Some(slot) = axis_slot(object.guid, &has_axis) else { continue };
            if has_axis[slot] {
                continue;
            }
            has_axis[slot] = true;
            ((slot * 4) as u32, DIDOI_ASPECTPOSITION)
        } else if object.ty & DIDFT_POV != 0 && pov < 4 {
            let offset = (32 + pov * 4) as u32;
            pov += 1;
            (offset, 0)
        } else if object.ty & DIDFT_BUTTON != 0 {
            let button = ((object.ty >> 8) & 0xFFFF) as usize;
            if button >= buttons.len() || buttons[button] {
                continue;
            }
            buttons[button] = true;
            ((48 + button) as u32, 0)
        } else {
            continue;
        };
        objs.push(DIOBJECTDATAFORMAT { pguid: std::ptr::null(), dwOfs: offset, dwType: object.ty, dwFlags: flags });
    }
    (objs, has_axis)
}

/// `MAKEDIPROP(n)`: DirectInput's own properties are numbers passed where a GUID's address goes.
fn prop(n: usize) -> *const GUID {
    n as *const GUID
}

fn window() -> HWND {
    unsafe {
        let w = GetForegroundWindow();
        if !w.0.is_null() { w } else { GetDesktopWindow() }
    }
}

fn create() -> Option<IDirectInput8W> {
    unsafe {
        let hinst: HINSTANCE = GetModuleHandleW(None).ok()?.into();
        let mut p: *mut core::ffi::c_void = std::ptr::null_mut();
        DirectInput8Create(hinst, DIRECTINPUT_VERSION, &IDirectInput8W::IID, &mut p, None).ok()?;
        (!p.is_null()).then(|| IDirectInput8W::from_raw(p))
    }
}

fn open(di: &IDirectInput8W, guid: &GUID, name: &str) -> Option<Open> {
    unsafe {
        let mut dev: Option<IDirectInputDevice8W> = None;
        di.CreateDevice(guid, &mut dev, None).ok()?;
        let dev = dev?;
        let mut objects = Vec::new();
        dev.EnumObjects(Some(collect_object), &mut objects as *mut _ as *mut core::ffi::c_void, DIDFT_ALL).ok()?;
        let (mut objs, has_axis) = format_objects(&objects);
        if objs.is_empty() {
            return None;
        }
        let mut fmt = DIDATAFORMAT {
            dwSize: std::mem::size_of::<DIDATAFORMAT>() as u32,
            dwObjSize: std::mem::size_of::<DIOBJECTDATAFORMAT>() as u32,
            dwFlags: if has_axis.iter().any(|&a| a) { DIDF_ABSAXIS } else { 0 },
            dwDataSize: std::mem::size_of::<RawState>() as u32,
            dwNumObjs: objs.len() as u32,
            rgodf: objs.as_mut_ptr(),
        };
        dev.SetDataFormat(&mut fmt).ok()?;
        // never take the device from the game or the wheel's own software: a look only
        dev.SetCooperativeLevel(window(), DISCL_NONEXCLUSIVE | DISCL_BACKGROUND).ok()?;
        let mut range = DIPROPRANGE { diph: DIPROPHEADER { dwSize: std::mem::size_of::<DIPROPRANGE>() as u32, dwHeaderSize: std::mem::size_of::<DIPROPHEADER>() as u32, dwObj: 0, dwHow: DIPH_DEVICE }, lMin: -RANGE, lMax: RANGE };
        let _ = dev.SetProperty(prop(4), &mut range.diph);
        let mut ranges = [(-RANGE, RANGE); 8];
        for k in (0..8).filter(|k| has_axis[*k]) {
            let header = DIPROPHEADER { dwSize: std::mem::size_of::<DIPROPRANGE>() as u32, dwHeaderSize: std::mem::size_of::<DIPROPHEADER>() as u32, dwObj: (k * 4) as u32, dwHow: DIPH_BYOFFSET };
            let mut asked = DIPROPRANGE { diph: header, lMin: -RANGE, lMax: RANGE };
            let _ = dev.SetProperty(prop(4), &mut asked.diph);
            let mut took = DIPROPRANGE { diph: header, lMin: 0, lMax: 0 };
            if dev.GetProperty(prop(4), &mut took.diph).is_ok() && took.lMax > took.lMin {
                ranges[k] = (took.lMin, took.lMax);
            }
        }
        // no dead zone and no saturation of the driver's own, as the game clears them
        for (p_id, v) in [(5usize, 0u32), (6, 10_000)] {
            let mut d = DIPROPDWORD { diph: DIPROPHEADER { dwSize: std::mem::size_of::<DIPROPDWORD>() as u32, dwHeaderSize: std::mem::size_of::<DIPROPHEADER>() as u32, dwObj: 0, dwHow: DIPH_DEVICE }, dwData: v };
            let _ = dev.SetProperty(prop(p_id), &mut d.diph);
        }
        let _ = dev.Acquire();
        Some(Open { name: name.to_string(), guid: *guid, dev, has_axis, ranges })
    }
}

fn same_name(a: &str, b: &str) -> bool {
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    a == b || a.starts_with(&b) || b.starts_with(&a)
}

/// The game controllers DirectInput lists now, by the names the game shows. Looked up afresh: some drivers take a moment, so ask
/// every second or two, not every frame.
pub fn connected() -> Vec<String> {
    PADS.with(|p| {
        let mut p = p.borrow_mut();
        if p.di.is_none() {
            p.di = create();
        }
        let Some(di) = p.di.clone() else { return vec![] };
        let mut v: Vec<(GUID, String)> = Vec::new();
        unsafe {
            let _ = di.EnumDevices(DI8DEVCLASS_ALL, Some(collect), &mut v as *mut _ as *mut core::ffi::c_void, DIEDFL_ATTACHEDONLY);
        }
        p.open.retain(|o| v.iter().any(|(g, _)| *g == o.guid));
        p.found = v.clone();
        let mut names: Vec<String> = vec![];
        for (_, n) in v {
            if !names.contains(&n) {
                names.push(n);
            }
        }
        names
    })
}

/// What the named device reports now: its eight axes from 0 to 1 (None for an axis it does not have) and which of its 128 buttons
/// are down.
pub fn live(name: &str) -> Option<([Option<f32>; 8], u128)> {
    PADS.with(|p| {
        let mut p = p.borrow_mut();
        let di = p.di.clone()?;
        if !p.open.iter().any(|o| same_name(&o.name, name)) {
            let (guid, found) = p.found.iter().find(|(_, n)| same_name(n, name)).cloned()?;
            let o = open(&di, &guid, &found)?;
            p.open.push(o);
        }
        let o = p.open.iter().find(|o| same_name(&o.name, name))?;
        let mut state = RawState { axes: [0; 8], pov: [u32::MAX; 4], buttons: [0; 128] };
        unsafe {
            if o.dev.Poll().is_err() || o.dev.GetDeviceState(std::mem::size_of::<RawState>() as u32, &mut state as *mut _ as *mut core::ffi::c_void).is_err() {
                let _ = o.dev.Acquire();
                o.dev.Poll().ok()?;
                o.dev.GetDeviceState(std::mem::size_of::<RawState>() as u32, &mut state as *mut _ as *mut core::ffi::c_void).ok()?;
            }
        }
        let axes = std::array::from_fn(|k| {
            o.has_axis[k].then(|| {
                let (min, max) = o.ranges[k];
                (((state.axes[k] as f64 - min as f64) / (max as f64 - min as f64)).clamp(0.0, 1.0)) as f32
            })
        });
        let mut buttons = 0u128;
        for (b, v) in state.buttons.iter().enumerate() {
            if v & 0x80 != 0 {
                buttons |= 1 << b;
            }
        }
        Some((axes, buttons))
    })
}

/// The axis slot (as in `gamectrler.cfg`) of the first axis that drives the wheel's motor.
fn ff_slot(objects: &[InputObject]) -> Option<usize> {
    let mut has = [false; 8];
    let mut first = None;
    for o in objects.iter().filter(|o| o.ty & DIDFT_AXIS != 0) {
        let Some(slot) = axis_slot(o.guid, &has) else { continue };
        if has[slot] {
            continue;
        }
        has[slot] = true;
        if o.flags & DIDOI_FFACTUATOR != 0 && first.is_none() {
            first = Some(slot);
        }
    }
    first
}

/// The device let go of, so the force-feedback test can have it to itself (the look at its axes is taken up again afterwards).
pub fn release(name: &str) {
    PADS.with(|p| p.borrow_mut().open.retain(|o| !same_name(&o.name, name)));
}

/// A wheel held for the direction test: the device to itself, with one constant force on its steering axis.
pub struct Ffb {
    dev: IDirectInputDevice8W,
    effect: IDirectInputEffect,
    slot: usize,
    range: (i32, i32),
    ff_offset: u32,
    flipped: bool,
}

/// Opens the wheel `name` exclusively for the test; fails with a sentence when it cannot be done.
pub fn ffb_open(name: &str, steer_axis: usize) -> Result<Ffb, String> {
    release(name);
    PADS.with(|p| {
        let mut p = p.borrow_mut();
        if p.di.is_none() {
            p.di = create();
        }
        let di = p.di.clone().ok_or("DirectInput is not available")?;
        let (guid, _) = p.found.iter().find(|(_, n)| same_name(n, name)).cloned().ok_or("The wheel is not connected")?;
        unsafe {
            let mut dev: Option<IDirectInputDevice8W> = None;
            di.CreateDevice(&guid, &mut dev, None).map_err(|e| e.to_string())?;
            let dev = dev.ok_or("The wheel could not be opened")?;
            let mut info = DIDEVICEINSTANCEW { dwSize: std::mem::size_of::<DIDEVICEINSTANCEW>() as u32, ..Default::default() };
            let hardware = dev.GetDeviceInfo(&mut info).ok().map(|_| (info.guidProduct.data1 as u16, (info.guidProduct.data1 >> 16) as u16));
            let flipped = hardware.is_some_and(|(vendor, product)| vendor == 0x046D && [0xC266u16, 0xC267, 0xC26D, 0xC26E].contains(&product));
            let mut objects = Vec::new();
            dev.EnumObjects(Some(collect_object), &mut objects as *mut _ as *mut core::ffi::c_void, DIDFT_ALL).map_err(|e| e.to_string())?;
            let slot = ff_slot(&objects).ok_or("This wheel has no force feedback")?;
            if slot != steer_axis {
                return Err("The steering axis is not the one that has the force feedback".into());
            }
            let (mut objs, has_axis) = format_objects(&objects);
            let mut fmt = DIDATAFORMAT {
                dwSize: std::mem::size_of::<DIDATAFORMAT>() as u32,
                dwObjSize: std::mem::size_of::<DIOBJECTDATAFORMAT>() as u32,
                dwFlags: if has_axis.iter().any(|&a| a) { DIDF_ABSAXIS } else { 0 },
                dwDataSize: std::mem::size_of::<RawState>() as u32,
                dwNumObjs: objs.len() as u32,
                rgodf: objs.as_mut_ptr(),
            };
            dev.SetDataFormat(&mut fmt).map_err(|e| e.to_string())?;
            dev.SetCooperativeLevel(window(), DISCL_EXCLUSIVE | DISCL_FOREGROUND).map_err(|_| "The wheel is held by another program (the wheel's own software, or the game): close it and try again")?;
            let mut range = DIPROPRANGE { diph: DIPROPHEADER { dwSize: std::mem::size_of::<DIPROPRANGE>() as u32, dwHeaderSize: std::mem::size_of::<DIPROPHEADER>() as u32, dwObj: 0, dwHow: DIPH_DEVICE }, lMin: -RANGE, lMax: RANGE };
            let _ = dev.SetProperty(prop(4), &mut range.diph);
            let header = DIPROPHEADER { dwSize: std::mem::size_of::<DIPROPRANGE>() as u32, dwHeaderSize: std::mem::size_of::<DIPROPHEADER>() as u32, dwObj: (slot * 4) as u32, dwHow: DIPH_BYOFFSET };
            let mut took = DIPROPRANGE { diph: header, lMin: 0, lMax: 0 };
            let mut r = (-RANGE, RANGE);
            if dev.GetProperty(prop(4), &mut took.diph).is_ok() && took.lMax > took.lMin {
                r = (took.lMin, took.lMax);
            }
            for (p_id, v) in [(5usize, 0u32), (6, 10_000), (9, DIPROPAUTOCENTER_OFF)] {
                let mut d = DIPROPDWORD { diph: DIPROPHEADER { dwSize: std::mem::size_of::<DIPROPDWORD>() as u32, dwHeaderSize: std::mem::size_of::<DIPROPHEADER>() as u32, dwObj: 0, dwHow: DIPH_DEVICE }, dwData: v };
                let _ = dev.SetProperty(prop(p_id), &mut d.diph);
            }
            dev.Acquire().map_err(|_| "The wheel could not be taken: bring this window to the front and try again")?;
            let ff_offset = (slot * 4) as u32;
            let mut axes = [ff_offset];
            let mut dirs = [0i32];
            let mut cf = DICONSTANTFORCE { lMagnitude: 0 };
            let mut eff = DIEFFECT {
                dwSize: std::mem::size_of::<DIEFFECT>() as u32,
                dwFlags: DIEFF_CARTESIAN | DIEFF_OBJECTOFFSETS,
                dwDuration: u32::MAX,
                dwGain: DI_FFNOMINALMAX,
                dwTriggerButton: DIEB_NOTRIGGER,
                cAxes: 1,
                rgdwAxes: axes.as_mut_ptr(),
                rglDirection: dirs.as_mut_ptr(),
                cbTypeSpecificParams: std::mem::size_of::<DICONSTANTFORCE>() as u32,
                lpvTypeSpecificParams: &mut cf as *mut _ as *mut core::ffi::c_void,
                ..Default::default()
            };
            let mut e: Option<IDirectInputEffect> = None;
            dev.CreateEffect(&GUID_ConstantForce, &mut eff, &mut e, None).map_err(|_| "The wheel's constant force could not be made")?;
            let effect = e.ok_or("The wheel's constant force could not be made")?;
            effect.Start(1, 0).map_err(|e| e.to_string())?;
            Ok(Ffb { dev, effect, slot, range: r, ff_offset, flipped })
        }
    })
}

impl Ffb {
    /// The steering axis from -1 to 1.
    pub fn position(&self) -> Option<f32> {
        let mut state = RawState { axes: [0; 8], pov: [u32::MAX; 4], buttons: [0; 128] };
        unsafe {
            if self.dev.Poll().is_err() || self.dev.GetDeviceState(std::mem::size_of::<RawState>() as u32, &mut state as *mut _ as *mut core::ffi::c_void).is_err() {
                return None;
            }
        }
        let (min, max) = self.range;
        Some((((state.axes[self.slot] as f64 - min as f64) / (max as f64 - min as f64)) * 2.0 - 1.0).clamp(-1.0, 1.0) as f32)
    }

    /// One short force (of at most `MAX_PULSE_FORCE`), timed by the wheel itself.
    pub fn pulse(&self, force: f32) -> bool {
        let limit = crate::ffb_calibration::MAX_PULSE_FORCE;
        let force = if self.flipped { -force } else { force };
        let mut constant = DICONSTANTFORCE { lMagnitude: (force.clamp(-limit, limit) * DI_FFNOMINALMAX as f32) as i32 };
        let mut axes = [self.ff_offset];
        let mut direction = [0i32];
        let mut effect = DIEFFECT {
            dwSize: std::mem::size_of::<DIEFFECT>() as u32,
            dwFlags: DIEFF_CARTESIAN | DIEFF_OBJECTOFFSETS,
            dwDuration: crate::ffb_calibration::PULSE_MS * 1000,
            cAxes: 1,
            rgdwAxes: axes.as_mut_ptr(),
            rglDirection: direction.as_mut_ptr(),
            cbTypeSpecificParams: std::mem::size_of::<DICONSTANTFORCE>() as u32,
            lpvTypeSpecificParams: &mut constant as *mut _ as *mut core::ffi::c_void,
            ..Default::default()
        };
        unsafe { self.effect.Stop().and_then(|_| self.effect.SetParameters(&mut effect, DIEP_DURATION | DIEP_TYPESPECIFICPARAMS | DIEP_START)).is_ok() }
    }
}

impl Drop for Ffb {
    fn drop(&mut self) {
        unsafe {
            let _ = self.effect.Stop();
            let _ = self.dev.Unacquire();
        }
    }
}
