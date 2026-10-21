//! Development-only, app-local generic-password storage. This does not access
//! the host keychain and is NOT secure credential storage: it persists a plist
//! inside the guest app's Documents directory, for this game's saved counters.
use crate::dyld::{export_c_func, HostConstant, ConstantExports, FunctionExports};
use crate::frameworks::foundation::{NSUInteger, ns_string::{get_static_str, from_rust_string}};
use crate::objc::{id, nil, msg, msg_class, retain, release};
use crate::mem::MutPtr;
use crate::Environment;
#[derive(Default)]
pub struct State { items: Option<id> }
const ATTRS: &[&str] = &["kSecClass", "kSecAttrAccount", "kSecAttrService", "kSecAttrGeneric", "kSecAttrAccessGroup", "kSecAttrLabel"];
fn value(env: &mut Environment, dict: id, name: &'static str) -> id {
    let key=get_static_str(env,name);msg![env; dict objectForKey:key]
}
fn path(env: &mut Environment) -> id {
    let p=env.fs.home_directory().join("Documents/SkeeBallGuestKeychain.plist");
    from_rust_string(env,p.as_str().to_string())
}
fn store(env: &mut Environment) -> id {
    if let Some(items)=env.framework_state.security.items { return items; }
    let p=path(env);let saved:id=msg_class![env; NSArray arrayWithContentsOfFile:p];release(env,p);
    let items:id=msg_class![env; NSMutableArray new];
    if !saved.is_null() {
        let count:NSUInteger=msg![env; saved count];
        for index in 0..count {
            let old:id=msg![env; saved objectAtIndex:index];
            let item:id=msg![env; old mutableCopy];
            ()=msg![env; items addObject:item];release(env,item);
        }
    }
    env.framework_state.security.items=Some(items);items
}
fn save(env: &mut Environment, items: id) -> bool {
    let p=path(env);let ok=msg![env; items writeToFile:p atomically:true];release(env,p);ok
}
fn matches(env: &mut Environment, item: id, query: id) -> bool {
    for name in ATTRS {
        let wanted=value(env,query,name);
        if !wanted.is_null() {
            let actual=value(env,item,name);
            let equal:bool=msg![env; wanted isEqual:actual];if !equal {return false;}
        }
    }
    true
}
fn find(env: &mut Environment, query: id) -> Option<(NSUInteger,id)> {
    let items=store(env);let count:NSUInteger=msg![env; items count];
    for index in 0..count {let item:id=msg![env; items objectAtIndex:index];if matches(env,item,query) {return Some((index,item));}}
    None
}
fn requested(env: &mut Environment, query: id, name: &'static str) -> bool {
    let v=value(env,query,name);if v.is_null() {false} else {msg![env; v boolValue]}
}
fn result(env: &mut Environment, item: id, query: id, out: MutPtr<id>) {
    if out.is_null() {return;}
    let data=requested(env,query,"kSecReturnData");
    let attrs=requested(env,query,"kSecReturnAttributes");
    let object=if attrs {
        let copy:id=msg![env; item mutableCopy];
        if !data {let key=get_static_str(env,"kSecValueData");()=msg![env; copy removeObjectForKey:key];}
        copy
    } else if data {let v=value(env,item,"kSecValueData");retain(env,v)} else {nil};
    env.mem.write(out,object);
}
fn SecItemCopyMatching(env: &mut Environment, query: id, out: MutPtr<id>) -> i32 {
    if !out.is_null() {env.mem.write(out,nil);}
    match find(env,query) {Some((_,item))=>{result(env,item,query,out);0},None=>-25300}
}
fn SecItemAdd(env: &mut Environment, attrs: id, out: MutPtr<id>) -> i32 {
    if !out.is_null() {env.mem.write(out,nil);}
    let class=value(env,attrs,"kSecClass");let generic=get_static_str(env,"kSecClassGenericPassword");
    let supported:bool=msg![env; class isEqual:generic];if !supported {return -50;}
    // Uniqueness is class/service/account/access-group, not the payload/label.
    let query:id=msg_class![env; NSMutableDictionary new];
    for name in ["kSecClass","kSecAttrAccount","kSecAttrService","kSecAttrAccessGroup"] {
        let v=value(env,attrs,name);if !v.is_null() {let key=get_static_str(env,name);()=msg![env; query setObject:v forKey:key];}
    }
    let exists=find(env,query).is_some();release(env,query);if exists {return -25299;}
    let item:id=msg_class![env; NSMutableDictionary new];
    for name in ATTRS.iter().copied().chain(std::iter::once("kSecValueData")) {
        let v=value(env,attrs,name);if !v.is_null() {let key=get_static_str(env,name);()=msg![env; item setObject:v forKey:key];}
    }
    let items=store(env);()=msg![env; items addObject:item];result(env,item,attrs,out);release(env,item);
    if save(env,items) {0} else {-36}
}
fn SecItemUpdate(env: &mut Environment, query: id, updates: id) -> i32 {
    let Some((_,item))=find(env,query) else {return -25300;};
    for name in ATTRS.iter().copied().chain(std::iter::once("kSecValueData")) {
        let v=value(env,updates,name);if !v.is_null() {let key=get_static_str(env,name);()=msg![env; item setObject:v forKey:key];}
    }
    let items=store(env);if save(env,items) {0} else {-36}
}
fn SecItemDelete(env: &mut Environment, query: id) -> i32 {
    let items=store(env);let mut removed=false;
    while let Some((index,_))=find(env,query) {()=msg![env; items removeObjectAtIndex:index];removed=true;}
    if !removed {-25300} else if save(env,items) {0} else {-36}
}
pub const CONSTANTS: ConstantExports = &[
    ("_kSecMatchLimitOne", HostConstant::NSString("kSecMatchLimitOne")),
    ("_kSecAttrAccount", HostConstant::NSString("kSecAttrAccount")),
    ("_kSecAttrService", HostConstant::NSString("kSecAttrService")),
    ("_kSecAttrGeneric", HostConstant::NSString("kSecAttrGeneric")),
    ("_kSecAttrAccessGroup", HostConstant::NSString("kSecAttrAccessGroup")),
    ("_kSecAttrLabel", HostConstant::NSString("kSecAttrLabel")),
    ("_kSecMatchLimit", HostConstant::NSString("kSecMatchLimit")),
    ("_kSecReturnData", HostConstant::NSString("kSecReturnData")),
    ("_kSecValueData", HostConstant::NSString("kSecValueData")),
    ("_kSecClass", HostConstant::NSString("kSecClass")),
    ("_kSecReturnAttributes", HostConstant::NSString("kSecReturnAttributes")),
    ("_kSecClassGenericPassword", HostConstant::NSString("kSecClassGenericPassword")),
 ];
pub const FUNCTIONS: FunctionExports = &[export_c_func!(SecItemCopyMatching(_, _)),export_c_func!(SecItemAdd(_, _)),export_c_func!(SecItemUpdate(_, _)),export_c_func!(SecItemDelete(_))];
pub const DYLIB: crate::dyld::HostDylib = crate::dyld::HostDylib {
path: "/System/Library/Frameworks/Security.framework/Security", aliases: &[], class_exports: &[], constant_exports: &[CONSTANTS], function_exports: &[FUNCTIONS],
};
