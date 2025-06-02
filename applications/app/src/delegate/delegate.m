#import <objc/runtime.h>
#import <objc/message.h>
#import <Foundation/Foundation.h>

static BOOL gHandlingSendEvent = NO;

BOOL isHandlingSendEvent(id self, SEL cmd) 
{
    (void)self;
    (void)cmd;

    return gHandlingSendEvent;
}

void setHandlingSendEvent(id self, SEL cmd, BOOL value) 
{
    (void)self;
    (void)cmd;

    gHandlingSendEvent = value;
}

BOOL injectDelegate(void) 
{
    Class application = objc_getClass("WinitApplication");
    if (!application) 
    {
        return NO;
    }

    {
        SEL sel = sel_registerName("isHandlingSendEvent");
        if (!class_respondsToSelector(application, sel)) 
        {
            class_addMethod(application, sel, (IMP)isHandlingSendEvent, "c@:");
        }
    }

    {
        SEL sel = sel_registerName("setHandlingSendEvent:");
        if (!class_respondsToSelector(application, sel)) 
        {
            class_addMethod(application, sel, (IMP)setHandlingSendEvent, "v@:c");
        }
    }

    return YES;
}
