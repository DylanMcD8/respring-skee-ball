// Minimal table-cell state required by startup SDK swizzling.
// Table rendering and datasource behavior remain intentionally unimplemented.
use crate::objc::{id, impl_HostObject_with_superclass, objc_classes, ClassExports, NSZonePtr};
#[derive(Default)]
struct Cell {
    superclass: super::ui_view::UIViewHostObject,
    selected: bool,
    highlighted: bool,
}
impl_HostObject_with_superclass!(Cell);
pub const CLASSES: ClassExports = objc_classes! {
(env, this, _cmd);
@implementation UITableViewCell: UIView
+ (id)allocWithZone:(NSZonePtr)_zone {
    env.objc.alloc_object(this,Box::<Cell>::default(),&mut env.mem)
}
- (bool)isSelected { env.objc.borrow::<Cell>(this).selected }
- (bool)isHighlighted { env.objc.borrow::<Cell>(this).highlighted }
- (())setSelected:(bool)value { env.objc.borrow_mut::<Cell>(this).selected=value; }
- (())setHighlighted:(bool)value { env.objc.borrow_mut::<Cell>(this).highlighted=value; }
- (())setSelected:(bool)value animated:(bool)_animated { env.objc.borrow_mut::<Cell>(this).selected=value; }
- (())setHighlighted:(bool)value animated:(bool)_animated { env.objc.borrow_mut::<Cell>(this).highlighted=value; }
@end
@implementation UITableView: UIScrollView
@end
@implementation UITableViewController: UIViewController
@end
};
