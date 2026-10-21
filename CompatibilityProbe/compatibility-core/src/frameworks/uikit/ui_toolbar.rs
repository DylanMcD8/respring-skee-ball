//! State for Unity's initially hidden keyboard accessory bars.
//! Rendering and interaction of toolbar items are not implemented yet.
use crate::objc::{id, impl_HostObject_with_superclass, objc_classes, ClassExports, NSZonePtr, HostObject, SEL, retain, release, msg_super};
use crate::frameworks::foundation::NSInteger;
#[derive(Default)]
struct Item { custom_view: id, system: NSInteger, target: id, action: Option<SEL> }
impl HostObject for Item {}
#[derive(Default)]
struct Toolbar {
    superclass: super::ui_view::UIViewHostObject,
    items: id,
    style: NSInteger,
    translucent: bool,
}
impl_HostObject_with_superclass!(Toolbar);
pub const CLASSES: ClassExports = objc_classes! {
(env, this, _cmd);
@implementation UIBarButtonItem: NSObject
+ (id)allocWithZone:(NSZonePtr)_zone { env.objc.alloc_object(this, Box::<Item>::default(), &mut env.mem) }
- (id)initWithBarButtonSystemItem:(NSInteger)system target:(id)target action:(SEL)action {
    let item=env.objc.borrow_mut::<Item>(this);
    item.system=system;item.target=target;item.action=Some(action);this
}
- (id)initWithCustomView:(id)view {
    let view=retain(env,view);env.objc.borrow_mut::<Item>(this).custom_view=view;this
}
- (id)customView { env.objc.borrow::<Item>(this).custom_view }
- (())dealloc {
    let view=env.objc.borrow::<Item>(this).custom_view;release(env,view);env.objc.dealloc_object(this,&mut env.mem);
}
- (id)target { env.objc.borrow::<Item>(this).target }
- (())setTarget:(id)target { env.objc.borrow_mut::<Item>(this).target=target; }
@end
@implementation UIToolbar: UIView
+ (id)allocWithZone:(NSZonePtr)_zone { env.objc.alloc_object(this, Box::<Toolbar>::default(), &mut env.mem) }
- (())setItems:(id)items {
    let items=retain(env,items);let old=env.objc.borrow::<Toolbar>(this).items;
    env.objc.borrow_mut::<Toolbar>(this).items=items;release(env,old);
}
- (id)items { env.objc.borrow::<Toolbar>(this).items }
- (())setBarStyle:(NSInteger)style { env.objc.borrow_mut::<Toolbar>(this).style=style; }
- (NSInteger)barStyle { env.objc.borrow::<Toolbar>(this).style }
- (())setTranslucent:(bool)value { env.objc.borrow_mut::<Toolbar>(this).translucent=value; }
- (bool)isTranslucent { env.objc.borrow::<Toolbar>(this).translucent }
- (())dealloc {
    let old=env.objc.borrow::<Toolbar>(this).items;release(env,old);
    () = msg_super![env; this dealloc];
}
@end
};
