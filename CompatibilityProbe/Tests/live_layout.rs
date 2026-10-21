#![allow(non_snake_case,dead_code)]
mod gles {
 pub mod gles11_raw { pub const MATRIX_MODE:u32=0x0BA0; pub const PROJECTION:u32=0x1701; }
 pub trait GLES { unsafe fn Viewport(&mut self,x:i32,y:i32,w:i32,h:i32); unsafe fn Scissor(&mut self,x:i32,y:i32,w:i32,h:i32); unsafe fn GetIntegerv(&mut self,p:u32,v:*mut i32); }
}
#[path="../compatibility-core/src/frameworks/opengles/skee_layout.rs"] mod layout;
#[derive(Default)] struct Mock { viewport:[i32;4], scissor:[i32;4] }
impl gles::GLES for Mock {
 unsafe fn Viewport(&mut self,x:i32,y:i32,w:i32,h:i32){self.viewport=[x,y,w,h];}
 unsafe fn Scissor(&mut self,x:i32,y:i32,w:i32,h:i32){self.scissor=[x,y,w,h];}
 unsafe fn GetIntegerv(&mut self,_:u32,v:*mut i32){*v=gles::gles11_raw::PROJECTION as i32;}
}
fn main(){
 std::env::set_var("SKEEBALL_WORLD_HEIGHT","480");
 let mut gl=Mock::default();
 for (w,h) in [(2853,2007),(2007,2853),(1398,2034),(2034,1398),(2853,2007)] {
  layout::update_size(w,h); let ww=layout::width();let hh=layout::height();
  assert!(ww>=320 && hh>=480);
  assert!((ww as f64/hh as f64-w as f64/h as f64).abs()<0.002);
  assert!(!layout::update_size(w,h));
  assert_eq!(layout::storage_width(320,480),ww); assert_eq!(layout::storage_height(320,480),hh);
  let mut l=layout::Layout::default(); unsafe{l.apply(&mut gl,4);}
  assert_eq!(gl.viewport,[(ww-320)/2*4,(hh-480)/2*4,1280,1920]);
  l.perspective=true;unsafe{l.apply(&mut gl,4);}
  assert_eq!(gl.viewport,[0,0,ww*4,hh*4]);assert_eq!(gl.scissor,gl.viewport);
 }
 assert!(!layout::update_size(0,0));
 println!("Repeated fold/rotation sizes, stable-size no-op, storage and world/UI rectangles passed");
}
