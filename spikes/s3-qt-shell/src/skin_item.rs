//! A `QQuickPaintedItem` implemented in Rust that paints a display list
//! built by `rvp_skin` (the classic BlackSpace window, or any `.dsf`).

use std::collections::BTreeMap;
use std::path::Path;
use std::pin::Pin;

use cxx_qt::CxxQtType;
use cxx_qt_lib::{QImage, QRect, QRectF, QString};
use rvp_skin::paint::{HAlign, Op, VAlign};
use rvp_skin::{DataModel, DisplayList, Skin, UiState, layout, paint};

use crate::ffi::bridge::{rvp_paint_fill, rvp_paint_image, rvp_paint_text, rvp_paint_translate};

#[cxx_qt::bridge]
pub mod qobject {
    unsafe extern "C++" {
        include!("cxx-qt-lib/qstring.h");
        type QString = cxx_qt_lib::QString;
        include!("cxx-qt-lib/qrectf.h");
        type QRectF = cxx_qt_lib::QRectF;
        include!("cxx-qt-lib/qrect.h");
        type QRect = cxx_qt_lib::QRect;
        include!("cxx-qt-lib/qpainter.h");
        type QPainter = cxx_qt_lib::QPainter;
    }

    unsafe extern "C++" {
        include!(<QtQuick/QQuickPaintedItem>);
        type QQuickPaintedItem;
    }

    unsafe extern "RustQt" {
        #[qobject]
        #[qml_element]
        #[base = QQuickPaintedItem]
        #[qproperty(QString, skin_path)]
        #[qproperty(QRectF, video_rect)]
        #[qproperty(QRectF, time_rect)]
        /// When set, the item covers only this part of the window and
        /// paints only the operations that touch it (a layer).
        #[qproperty(QRectF, region)]
        #[qproperty(QString, status)]
        #[qproperty(i32, ops_painted)]
        type SkinItem = super::SkinItemRust;

        /// Paints the cached display list (called on the render thread
        /// while the GUI thread is blocked).
        #[qinvokable]
        #[cxx_override]
        unsafe fn paint(self: Pin<&mut Self>, painter: *mut QPainter);

        /// Lays the skin out for a new size (GUI thread).
        #[qinvokable]
        fn relayout(self: Pin<&mut Self>, width: f64, height: f64);

        /// Repaints only the current-time label, as a real player would
        /// once per second.
        #[qinvokable]
        fn repaint_time(self: Pin<&mut Self>);

        /// "calls N mean X ms max Y ms" for the paint() calls so far.
        #[qinvokable]
        fn paint_stats(self: &Self) -> QString;

        /// `QQuickPaintedItem::update(rect)`; an empty rect repaints all.
        #[inherit]
        #[cxx_name = "update"]
        fn update_rect(self: Pin<&mut Self>, rect: &QRect);
    }

    // The generated default constructor passes a QObject* parent, which
    // QQuickPaintedItem does not accept; QML sets the parent item later.
    impl cxx_qt::Constructor<()> for SkinItem {}
}

impl cxx_qt::Constructor<()> for qobject::SkinItem {
    type NewArguments = ();
    type BaseArguments = ();
    type InitializeArguments = ();

    fn route_arguments(_arguments: ()) -> ((), (), ()) {
        ((), (), ())
    }

    fn new((): ()) -> SkinItemRust {
        SkinItemRust::default()
    }
}

#[derive(Default)]
pub struct SkinItemRust {
    skin_path: QString,
    video_rect: QRectF,
    time_rect: QRectF,
    region: QRectF,
    status: QString,
    paint_calls: u32,
    paint_total: std::time::Duration,
    paint_max: std::time::Duration,
    ops_painted: i32,
    skin: Option<Skin>,
    loaded_path: String,
    list: Option<DisplayList>,
    atlases: BTreeMap<String, Option<QImage>>,
}

fn rgba(color: [u8; 4]) -> u32 {
    u32::from_be_bytes(color)
}

impl qobject::SkinItem {
    fn relayout(mut self: Pin<&mut Self>, width: f64, height: f64) {
        let path = self.skin_path().to_string();
        if self.rust().loaded_path != path || self.rust().skin.is_none() {
            let loaded = Skin::open(Path::new(&path));
            let status = match &loaded {
                Ok(_) => format!("skin loaded: {path}"),
                Err(error) => format!("skin failed: {error}"),
            };
            {
                let mut rust = self.as_mut().rust_mut();
                rust.skin = loaded.ok();
                rust.loaded_path = path;
                rust.atlases.clear();
            }
            self.as_mut().set_status(QString::from(&status));
        }
        // Item sizes are window sizes, far inside i32.
        #[allow(clippy::cast_possible_truncation)]
        let (w, h) = (width.round() as i32, height.round() as i32);
        let result = self.rust().skin.as_ref().map(|skin| {
            let state = UiState::default();
            let data = DataModel::sample();
            layout(skin, "MainSkinArea", w, h, &state).map(|placed| {
                let video = placed.slot("CMD_VIDEO_WINDOW");
                let time = placed.find_command("CMD_CURRENTTIME_MESSAGE").map(|node| node.rect);
                (paint(skin, &placed, &state, &data), video, time)
            })
        });
        match result {
            Some(Ok((list, video, time))) => {
                self.as_mut().rust_mut().list = Some(list);
                if let Some(rect) = time {
                    self.as_mut().set_time_rect(QRectF::new(
                        f64::from(rect.x),
                        f64::from(rect.y),
                        f64::from(rect.w),
                        f64::from(rect.h),
                    ));
                }
                if let Some(rect) = video {
                    self.as_mut().set_video_rect(QRectF::new(
                        f64::from(rect.x),
                        f64::from(rect.y),
                        f64::from(rect.w),
                        f64::from(rect.h),
                    ));
                }
            }
            Some(Err(error)) => {
                self.as_mut()
                    .set_status(QString::from(&format!("layout failed: {error}")));
            }
            None => {}
        }
        self.update_rect(&QRect::default());
    }

    fn paint_stats(&self) -> QString {
        let calls = self.paint_calls.max(1);
        QString::from(&format!(
            "calls {} mean {:.2} ms max {:.2} ms",
            self.paint_calls,
            self.paint_total.as_secs_f64() * 1000.0 / f64::from(calls),
            self.paint_max.as_secs_f64() * 1000.0
        ))
    }

    fn repaint_time(self: Pin<&mut Self>) {
        let rect = self.time_rect();
        // Window coordinates, far inside i32.
        #[allow(clippy::cast_possible_truncation)]
        let rect = QRect::new(
            rect.x().floor() as i32,
            rect.y().floor() as i32,
            rect.width().ceil() as i32,
            rect.height().ceil() as i32,
        );
        self.update_rect(&rect);
    }

    /// # Safety
    /// `painter` comes from `QQuickPaintedItem::paint` and is valid for the
    /// duration of the call.
    unsafe fn paint(mut self: Pin<&mut Self>, painter: *mut qobject::QPainter) {
        let Some(painter) = (unsafe { painter.as_mut() }) else {
            return;
        };
        let mut painter = unsafe { Pin::new_unchecked(painter) };
        let started = std::time::Instant::now();
        let mut rust = self.as_mut().rust_mut();
        let rust = rust.as_mut().get_mut();
        let (Some(skin), Some(list)) = (rust.skin.as_ref(), rust.list.as_ref()) else {
            return;
        };
        // Layer mode: window coordinates shifted into the item, and only
        // operations that touch the region.
        #[allow(clippy::cast_possible_truncation)]
        let region = (!rust.region.is_empty()).then(|| {
            rvp_skin::Rect::new(
                rust.region.x().floor() as i32,
                rust.region.y().floor() as i32,
                rust.region.width().ceil() as i32,
                rust.region.height().ceil() as i32,
            )
        });
        if let Some(region) = region {
            rvp_paint_translate(painter.as_mut(), -region.x, -region.y);
        }
        let mut painted = 0;
        for op in &list.ops {
            if let Some(region) = region {
                let bounds = match op {
                    Op::Image { dst, clip, .. } => dst.intersect(clip),
                    Op::Fill { rect, .. } => *rect,
                    Op::Text(text) => text.rect.intersect(&text.clip),
                };
                if bounds.intersect(&region).is_empty() {
                    continue;
                }
            }
            match op {
                Op::Image {
                    atlas,
                    src,
                    dst,
                    clip,
                } => {
                    let image = rust.atlases.entry(atlas.to_ascii_lowercase()).or_insert_with(|| {
                        let png = skin.image(atlas)?.encode_png().ok()?;
                        QImage::from_data(&png, Some("PNG"))
                    });
                    if let Some(image) = image {
                        rvp_paint_image(
                            painter.as_mut(),
                            image,
                            src.x,
                            src.y,
                            src.w,
                            src.h,
                            dst.x,
                            dst.y,
                            dst.w,
                            dst.h,
                            clip.x,
                            clip.y,
                            clip.w,
                            clip.h,
                        );
                    }
                }
                Op::Fill { rect, color } => {
                    rvp_paint_fill(painter.as_mut(), rect.x, rect.y, rect.w, rect.h, rgba(*color));
                }
                Op::Text(text) => {
                    // Font sizes are a few pixels.
                    #[allow(clippy::cast_possible_truncation)]
                    let pixel_size = text.font.size_px.round() as i32;
                    rvp_paint_text(
                        painter.as_mut(),
                        &text.text,
                        text.trailing.as_deref().unwrap_or(""),
                        &text.font.family,
                        pixel_size,
                        text.font.bold,
                        rgba(text.color),
                        text.rect.x,
                        text.rect.y,
                        text.rect.w,
                        text.rect.h,
                        text.clip.x,
                        text.clip.y,
                        text.clip.w,
                        text.clip.h,
                        match text.h_align {
                            HAlign::Left => 0,
                            HAlign::Center => 1,
                            HAlign::Right => 2,
                        },
                        match text.v_align {
                            VAlign::Top => 0,
                            VAlign::Center => 1,
                            VAlign::Bottom => 2,
                        },
                        text.ellipsis,
                    );
                }
            }
            painted += 1;
        }
        rust.ops_painted = painted;
        let elapsed = started.elapsed();
        rust.paint_calls += 1;
        rust.paint_total += elapsed;
        rust.paint_max = rust.paint_max.max(elapsed);
    }
}
