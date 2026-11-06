// Two small things the Rust side can't reach through cxx-qt-lib: Qt's message handler (QML warnings and console
// lines go into Turbine's activity log, fatal errors into crash.log) and the clipboard.

#include <QtCore/QByteArray>
#include <QtCore/QString>
#include <QtCore/QtGlobal>
#include <QtGui/QClipboard>
#include <QtGui/QGuiApplication>

namespace {
using Sink = void (*)(int kind, const char *place, const char *text);
Sink s_sink = nullptr;
QtMessageHandler s_previous = nullptr;

void handler(QtMsgType type, const QMessageLogContext &ctx, const QString &msg)
{
    if (s_sink) {
        const int kind = type == QtDebugMsg ? 0 : type == QtWarningMsg ? 1 : type == QtCriticalMsg ? 2 : type == QtFatalMsg ? 3 : 4;
        QByteArray place;
        if (ctx.file && *ctx.file)
            place = QByteArray(ctx.file) + ':' + QByteArray::number(ctx.line);
        else if (ctx.category && qstrcmp(ctx.category, "default") != 0)
            place = QByteArray(ctx.category);
        const QByteArray text = msg.toUtf8();
        s_sink(kind, place.constData(), text.constData());
    }
    // Everything still goes where it went before (the terminal), so the build's checks see QML errors.
    if (s_previous)
        s_previous(type, ctx, msg);
}
}

extern "C" void turbine_install_qt_handler(Sink sink)
{
    s_sink = sink;
    s_previous = qInstallMessageHandler(handler);
}

extern "C" void turbine_copy_text(const char *text)
{
    if (QClipboard *clip = QGuiApplication::clipboard())
        clip->setText(QString::fromUtf8(text));
}

// The Game Mode status overlay. gamescope (SteamOS's Game Mode compositor) paints a window as an overlay over the
// focused app when the window has the X11 property GAMESCOPE_EXTERNAL_OVERLAY set to 1 (a CARDINAL); it never gives
// such a window input. This is how MangoHud's mangoapp does it: MangoHud src/app/main.cpp (commit 060f935), lines
// 444 to 466 (the property name, XInternAtom, then set_x_cardinal_property, which is XChangeProperty with XA_CARDINAL
// and format 32, lines 146 to 157), GAMESCOPE_NO_FOCUS at lines 587 to 591, and the property set back to 0 when the
// window hides at lines 652 to 657. Only Turbine's own overlay window (objectName "turbineHud") is ever touched.
// Returns 1 when done, -1 when the window isn't there, -2 when Qt isn't on X11, -3 when this build has no X11.
#include <QtGui/QWindow>
#include <QtGui/qtguiglobal.h>
#if QT_CONFIG(xcb) && __has_include(<X11/Xlib.h>)
#include <QtGui/qguiapplication_platform.h>
#include <X11/Xlib.h>
#include <X11/Xatom.h>
#define TURBINE_X11 1
#endif

extern "C" int turbine_overlay_mark(int on)
{
    QWindow *win = nullptr;
    const auto windows = QGuiApplication::allWindows();
    for (QWindow *w : windows) {
        if (w->objectName() == QLatin1String("turbineHud")) {
            win = w;
            break;
        }
    }
    if (!win)
        return -1;
#ifdef TURBINE_X11
    auto *x11 = qGuiApp->nativeInterface<QNativeInterface::QX11Application>();
    Display *dpy = x11 ? x11->display() : nullptr;
    if (!dpy)
        return -2;
    // The native window must exist before it is shown, so gamescope sees the property when it maps.
    win->create();
    const Window id = static_cast<Window>(win->winId());
    if (!id)
        return -1;
    // Format 32 properties are read from an array of long (Xlib), whatever long's size.
    long value = on ? 1 : 0;
    Atom overlay = XInternAtom(dpy, "GAMESCOPE_EXTERNAL_OVERLAY", False);
    XChangeProperty(dpy, id, overlay, XA_CARDINAL, 32, PropModeReplace, reinterpret_cast<unsigned char *>(&value), 1);
    if (on) {
        long one = 1;
        Atom noFocus = XInternAtom(dpy, "GAMESCOPE_NO_FOCUS", False);
        XChangeProperty(dpy, id, noFocus, XA_CARDINAL, 32, PropModeReplace, reinterpret_cast<unsigned char *>(&one), 1);
    }
    XFlush(dpy);
    return 1;
#else
    Q_UNUSED(on);
    return -3;
#endif
}
