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

BOOL injectSendEventInWinit(void) 
{
    Class winitClass = objc_getClass("WinitApplication");
    if (!winitClass) 
    {
        return NO;
    }

    {
        SEL sel = sel_registerName("isHandlingSendEvent");
        if (!class_respondsToSelector(winitClass, sel)) 
        {
            class_addMethod(winitClass, sel, (IMP)isHandlingSendEvent, "c@:");
        }
    }

    {
        SEL sel = sel_registerName("setHandlingSendEvent:");
        if (!class_respondsToSelector(winitClass, sel)) 
        {
            class_addMethod(winitClass, sel, (IMP)setHandlingSendEvent, "v@:c");
        }
    }

    return YES;
}
