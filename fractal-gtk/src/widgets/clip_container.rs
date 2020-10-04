// Adapted from https://gitlab.gnome.org/GNOME/libhandy/-/blob/1.0.0/src/hdy-window-mixin.c
// (C) 2020 Alexander Mikhaylenko
//
use gdk::prelude::*;
use glib::subclass;
use glib::subclass::prelude::*;
use glib::translate::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

use std::cell::{Cell, RefCell};
use std::cmp::max;
use std::collections::HashMap;

use crate::util::get_border_radius;

#[derive(Eq, PartialEq, Debug, Copy, Clone, Hash)]
pub enum Corner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

static CORNERS: [Corner; 4] = [
    Corner::TopLeft,
    Corner::TopRight,
    Corner::BottomLeft,
    Corner::BottomRight,
];

/// A simple container to clip it's children
pub struct ClipContainerPriv {
    last_border_radius: Cell<i32>,
    masks: RefCell<HashMap<Corner, cairo::Surface>>,
    content: RefCell<Option<gtk::Widget>>,
}

impl ObjectSubclass for ClipContainerPriv {
    const NAME: &'static str = "FrctlClipContainer";
    type ParentType = gtk::Bin;
    type Instance = subclass::simple::InstanceStruct<Self>;
    type Class = subclass::simple::ClassStruct<Self>;

    glib_object_subclass!();

    fn new() -> Self {
        Self {
            last_border_radius: Cell::new(0),
            masks: RefCell::new(HashMap::new()),
            content: RefCell::new(None),
        }
    }
}

impl ObjectImpl for ClipContainerPriv {
    glib_object_impl!();

    fn constructed(&self, obj: &glib::Object) {
        self.parent_constructed(obj);
        let widget = obj.downcast_ref::<gtk::Widget>().unwrap();
        widget.set_widget_name("clip-container");
    }
}

impl WidgetImpl for ClipContainerPriv {
    fn draw(&self, widget: &gtk::Widget, cr: &cairo::Context) -> glib::signal::Inhibit {
        let gdk_window = widget
            .get_window()
            .expect("Could not get gdk::Window from ClipContainer");
        if gtk::cairo_should_draw_window(cr, &gdk_window) {
            let maybe_clip = cr.get_clip_rectangle();
            let mut clip = gdk::Rectangle {
                x: 0,
                y: 0,
                width: 0,
                height: 0,
            };
            let ctx = widget.get_style_context();
            let width = widget.get_allocated_width();
            let height = widget.get_allocated_height();
            let w = width as f64;
            let h = height as f64;
            let radius = get_border_radius(&ctx);
            let r = radius as f64;
            let xy = 0.0;

            // No custom drawing with default radius
            if radius <= 0 {
                return self.parent_draw(widget, cr);
            }

            if let Some(c) = maybe_clip {
                clip = c;
            } else {
                clip.width = width;
                clip.height = height;
            }

            cr.save();

            let scale_factor = widget.get_scale_factor();
            if radius * scale_factor != self.last_border_radius.get() {
                create_masks(self, widget, cr, radius);
                self.last_border_radius.set(radius * scale_factor);
            }

            let surface = gdk_window
                .create_similar_surface(cairo::Content::ColorAlpha, max(width, 1), max(height, 1))
                .unwrap();
            let surface_ctx = cairo::Context::new(&surface);
            surface.set_device_offset(-clip.x as f64, -clip.y as f64);

            if !widget.get_app_paintable() {
                gtk::render_background(&ctx, &surface_ctx, xy, xy, w, h);
                gtk::render_frame(&ctx, &surface_ctx, xy, xy, w, h);
            }

            if let Some(child) = &*self.content.borrow() {
                widget
                    .downcast_ref::<gtk::Container>()
                    .unwrap()
                    .propagate_draw(child, &surface_ctx);
            }

            cr.set_source_surface(&surface, 0.0, 0.0);
            cr.rectangle(xy + r, xy, w - r * 2.0, r);
            cr.rectangle(xy + r, xy + h - r, w - r * 2.0, r);
            cr.rectangle(xy, xy + r, w, h - r * 2.0);
            cr.fill();

            if (clip.x as f64) < xy + r && (clip.y as f64) < xy + r {
                mask_corner(self, cr, scale_factor as f64, Corner::TopLeft, xy, xy);
            }

            if ((clip.x + clip.width) as f64) > xy + w - r && (clip.y as f64) < xy + r {
                mask_corner(
                    self,
                    cr,
                    scale_factor as f64,
                    Corner::TopRight,
                    xy + w - r,
                    xy,
                );
            }

            if (clip.x as f64) < xy + r && ((clip.y + clip.height) as f64) > xy + h - r {
                mask_corner(
                    self,
                    cr,
                    scale_factor as f64,
                    Corner::BottomLeft,
                    xy,
                    xy + h - r,
                );
            }

            if ((clip.x + clip.width) as f64) > xy + w - r
                && ((clip.y + clip.height) as f64) > xy + h - r
            {
                mask_corner(
                    self,
                    cr,
                    scale_factor as f64,
                    Corner::BottomRight,
                    xy + w - r,
                    xy + h - r,
                );
            }

            surface.flush();

            cr.restore();
        }
        // Propagate draw further
        glib::signal::Inhibit(false)
    }
}

impl ContainerImpl for ClipContainerPriv {
    fn add(&self, container: &gtk::Container, widget: &gtk::Widget) {
        self.parent_add(container, widget);
        self.content.replace(Some(widget.clone()));
    }
}
impl BinImpl for ClipContainerPriv {}

glib_wrapper! {
    pub struct ClipContainer(
        Object<subclass::simple::InstanceStruct<ClipContainerPriv>,
        subclass::simple::ClassStruct<ClipContainerPriv>,
        ClipContainerClass>) @ extends gtk::Widget, gtk::Container, gtk::Bin;
     match fn {
         get_type => || ClipContainerPriv::get_type().to_glib(),
     }
}

impl ClipContainer {
    pub fn new() -> Self {
        glib::Object::new(ClipContainer::static_type(), &[])
            .expect("Failed to initialize ClipContainer widget")
            .downcast()
            .expect("Failed to cast Object to ClipContainer")
    }
}

fn create_masks(
    container: &ClipContainerPriv,
    widget: &gtk::Widget,
    cr: &cairo::Context,
    radius: i32,
) {
    let scale_factor = widget.get_scale_factor();
    let r = radius as f64;
    let mut masks = container.masks.borrow_mut();

    masks.clear();

    if radius > 0 {
        for (i, corner) in CORNERS.iter().enumerate() {
            let surface = cr
                .get_target()
                .create_similar_image(
                    cairo::Format::A8,
                    radius * scale_factor,
                    radius * scale_factor,
                )
                .unwrap();

            let mask_ctx = cairo::Context::new(&surface);
            mask_ctx.scale(scale_factor.into(), scale_factor.into());
            mask_ctx.set_source_rgb(0.0, 0.0, 0.0);
            let mod_val = if i % 2 == 0 { r } else { 0.0 };
            let val = if i / 2 == 0 { r } else { 0.0 };
            mask_ctx.arc(mod_val, val, r, 0.0, std::f64::consts::PI * 2.0);
            mask_ctx.fill();

            masks.insert(*corner, surface);
        }
    }
}

fn mask_corner(
    container: &ClipContainerPriv,
    cr: &cairo::Context,
    scale_factor: f64,
    corner: Corner,
    x: f64,
    y: f64,
) {
    cr.save();
    cr.scale(1.0 / scale_factor, 1.0 / scale_factor);
    cr.mask_surface(
        container.masks.borrow().get(&corner).unwrap(),
        x * scale_factor,
        y * scale_factor,
    );
    cr.restore();
}
