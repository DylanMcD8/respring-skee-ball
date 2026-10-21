//! Serial background operation queues for legacy SDK work. The queue uses the
//! existing cooperative NSThread scheduler; it never runs work on the caller.
use crate::objc::{id, nil, msg, msg_class, objc_classes, HostObject, NSZonePtr, ClassExports, SEL, retain, release};
use super::{NSInteger, NSUInteger};
use std::time::Duration;
#[derive(Default)]
struct Operation { cancelled: bool, executing: bool, finished: bool, dependencies: Vec<id>, target: id, selector: Option<SEL>, argument: id }
impl HostObject for Operation {}
#[derive(Default)]
struct Queue { operations: Vec<id>, active: bool, suspended: bool, max: NSInteger }
impl HostObject for Queue {}
pub const CLASSES: ClassExports=objc_classes! {
(env,this,_cmd);
@implementation NSOperation: NSObject
+ (id)allocWithZone:(NSZonePtr)_zone {env.objc.alloc_object(this,Box::<Operation>::default(),&mut env.mem)}
- (bool)isCancelled {env.objc.borrow::<Operation>(this).cancelled}
- (bool)isExecuting {env.objc.borrow::<Operation>(this).executing}
- (bool)isFinished {env.objc.borrow::<Operation>(this).finished}
- (bool)isConcurrent {false}
- (bool)isReady {
    let dependencies=env.objc.borrow::<Operation>(this).dependencies.clone();
    dependencies.iter().all(|dep| {let dep=*dep;msg![env; dep isFinished]})
}
- (())cancel {env.objc.borrow_mut::<Operation>(this).cancelled=true;}
- (())main {}
- (())start {
    if !env.objc.borrow::<Operation>(this).cancelled {
        env.objc.borrow_mut::<Operation>(this).executing=true;()=msg![env; this main];
    }
    let op=env.objc.borrow_mut::<Operation>(this);op.executing=false;op.finished=true;
}
- (())addDependency:(id)other {let other=retain(env,other);env.objc.borrow_mut::<Operation>(this).dependencies.push(other);}
- (())waitUntilFinished {while !env.objc.borrow::<Operation>(this).finished {env.sleep(Duration::from_millis(1));}}
- (())dealloc {
    let op=std::mem::take(env.objc.borrow_mut::<Operation>(this));
    for dep in op.dependencies {release(env,dep);}release(env,op.target);release(env,op.argument);
    env.objc.dealloc_object(this,&mut env.mem);
}
@end
@implementation NSInvocationOperation: NSOperation
- (id)initWithTarget:(id)target selector:(SEL)selector object:(id)argument {
    let target=retain(env,target);let argument=retain(env,argument);
    let op=env.objc.borrow_mut::<Operation>(this);op.target=target;op.selector=Some(selector);op.argument=argument;this
}
- (())main {
    let op=env.objc.borrow::<Operation>(this);let (target,selector,argument)=(op.target,op.selector.unwrap(),op.argument);
    let _:id=msg![env; target performSelector:selector withObject:argument];
}
@end
@implementation NSOperationQueue: NSObject
+ (id)allocWithZone:(NSZonePtr)_zone {env.objc.alloc_object(this,Box::<Queue>::default(),&mut env.mem)}
- (())setMaxConcurrentOperationCount:(NSInteger)max {env.objc.borrow_mut::<Queue>(this).max=max;}
- (NSInteger)maxConcurrentOperationCount {env.objc.borrow::<Queue>(this).max}
- (())setSuspended:(bool)value {env.objc.borrow_mut::<Queue>(this).suspended=value;}
- (bool)isSuspended {env.objc.borrow::<Queue>(this).suspended}
- (NSUInteger)operationCount {env.objc.borrow::<Queue>(this).operations.len() as NSUInteger}
- (())addOperation:(id)operation {
    let operation=retain(env,operation);let queue=env.objc.borrow_mut::<Queue>(this);queue.operations.push(operation);
    if !queue.active {
        queue.active=true;
        let selector=env.objc.lookup_selector("_touchHLEDrainOperations:").unwrap();
        ()=msg_class![env; NSThread detachNewThreadSelector:selector toTarget:this withObject:nil];
    }
}
- (())_touchHLEDrainOperations:(id)_unused {
    loop {
        if env.objc.borrow::<Queue>(this).operations.is_empty() {break;}
        if env.objc.borrow::<Queue>(this).suspended {env.sleep(Duration::from_millis(1));continue;}
        let pending=env.objc.borrow::<Queue>(this).operations.clone();
        let ready=pending.iter().position(|op| {let op=*op;let cancelled:bool=msg![env; op isCancelled];cancelled || msg![env; op isReady]});
        let Some(index)=ready else {env.sleep(Duration::from_millis(1));continue;};
        let op=env.objc.borrow::<Queue>(this).operations[index];
        ()=msg![env; op start];
        while !{let done:bool=msg![env; op isFinished];done} {env.sleep(Duration::from_millis(1));}
        env.objc.borrow_mut::<Queue>(this).operations.remove(index);release(env,op);
    }
    env.objc.borrow_mut::<Queue>(this).active=false;
}
- (())cancelAllOperations {
    let pending=env.objc.borrow::<Queue>(this).operations.clone();for op in pending {()=msg![env; op cancel];}
}
- (())waitUntilAllOperationsAreFinished {while !env.objc.borrow::<Queue>(this).operations.is_empty() {env.sleep(Duration::from_millis(1));}}
- (())dealloc {
    let pending=std::mem::take(&mut env.objc.borrow_mut::<Queue>(this).operations);for op in pending {release(env,op);}
    env.objc.dealloc_object(this,&mut env.mem);
}
@end
};
