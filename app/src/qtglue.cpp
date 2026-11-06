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
