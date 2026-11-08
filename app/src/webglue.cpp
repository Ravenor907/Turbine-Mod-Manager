// Nexus inside Turbine: the parts of Qt WebEngine the Rust side can't reach through cxx-qt-lib.
// - turbine_web_prepare: before the application object exists, the nxm scheme is registered (Qt ignores it later)
//   and QtWebEngineQuick::initialize() runs. Both are only set up here; no web page is loaded.
// - turbine_web_profile: remembers where the browser keeps its files and registers the one profile Turbine uses as the
//   QML singleton TurbineWeb.NexusProfile. The profile itself is made the first time something uses it (opening the
//   browser, signing out), so Chromium never starts in a run that doesn't open Nexus.
// - NxmHandler: a Mod Manager Download link the page navigates to stops here (the page stays where it is) and its text
//   goes to Rust (browse.rs), which checks it before anything else happens.
// - turbine_web_sign_out, turbine_web_set_cookie: Settings' sign out, and the sign-in the browser helper brings back
//   from the player's own browser (nexusmods.com cookies only, checked in Rust first and again here).
// Cookie values never go to the log: only names and counts do.
// No class here has Q_OBJECT, so nothing in this file needs moc.

#include <QtCore/QByteArray>
#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QPointer>
#include <QtCore/QString>
#include <QtCore/QUrl>
#include <QtGui/QGuiApplication>
#include <QtNetwork/QNetworkCookie>
#include <QtQml/QJSEngine>
#include <QtQml/qqml.h>
#include <QtWebEngineCore/qwebenginecookiestore.h>
#include <QtWebEngineCore/qwebengineurlrequestjob.h>
#include <QtWebEngineCore/qwebengineurlscheme.h>
#include <QtWebEngineCore/qwebengineurlschemehandler.h>
#include <QtWebEngineQuick/qquickwebengineprofile.h>
#include <QtWebEngineQuick/qtwebenginequickglobal.h>

namespace {
using LogSink = void (*)(const char *text);
using NxmSink = void (*)(const char *url);

LogSink s_log = nullptr;
NxmSink s_nxm = nullptr;
QString s_data;
QString s_cache;
QPointer<QQuickWebEngineProfile> s_profile;

void say(const QString &text)
{
    if (s_log) {
        const QByteArray b = text.toUtf8();
        s_log(b.constData());
    }
}

// A Mod Manager Download link (nxm://...) the page navigated to. The request fails on purpose, so the page stays on
// the file page; the link's text goes to Rust, which checks it (nxm::parse) before using it.
class NxmHandler : public QWebEngineUrlSchemeHandler
{
public:
    explicit NxmHandler(QObject *parent) : QWebEngineUrlSchemeHandler(parent) { }
    void requestStarted(QWebEngineUrlRequestJob *job) override
    {
        const QByteArray url = job->requestUrl().toString(QUrl::FullyEncoded).toUtf8();
        job->fail(QWebEngineUrlRequestJob::RequestAborted);
        if (s_nxm && url.size() <= 4096)
            s_nxm(url.constData());
    }
};

QQuickWebEngineProfile *profile()
{
    if (s_profile)
        return s_profile;
    auto *p = new QQuickWebEngineProfile(qApp);
    // One profile kept on disk: the player signs in once and stays signed in. Profiles are off the record unless
    // told otherwise; session cookies are kept too (Nexus may sign in with one).
    p->setStorageName(QStringLiteral("nexus"));
    p->setOffTheRecord(false);
    if (!s_data.isEmpty()) {
        QDir().mkpath(s_data);
        p->setPersistentStoragePath(s_data);
    }
    if (!s_cache.isEmpty()) {
        QDir().mkpath(s_cache);
        p->setCachePath(s_cache);
    }
    p->setHttpCacheType(QQuickWebEngineProfile::DiskHttpCache);
    p->setPersistentCookiesPolicy(QQuickWebEngineProfile::ForcePersistentCookies);
    p->installUrlSchemeHandler(QByteArrayLiteral("nxm"), new NxmHandler(p));
    s_profile = p;
    say(QStringLiteral("Nexus browser profile ready (kept in %1; signed in stays signed in)").arg(p->persistentStoragePath()));
    return p;
}
}

// Before QGuiApplication: the nxm scheme, then WebEngine's own setup. Returns 1.
extern "C" int turbine_web_prepare(LogSink log)
{
    s_log = log;
    QWebEngineUrlScheme nxm(QByteArrayLiteral("nxm"));
    nxm.setSyntax(QWebEngineUrlScheme::Syntax::Path);
    nxm.setFlags({});
    QWebEngineUrlScheme::registerScheme(nxm);
    say(QStringLiteral("nxm scheme registered"));
    QtWebEngineQuick::initialize();
    say(QStringLiteral("WebEngine ready (before the app)"));
    return 1;
}

// After QGuiApplication, before Main.qml: where the profile keeps its files, and TurbineWeb.NexusProfile for QML.
extern "C" void turbine_web_profile(const char *data_dir, const char *cache_dir, NxmSink nxm)
{
    s_data = QString::fromUtf8(data_dir);
    s_cache = QString::fromUtf8(cache_dir);
    s_nxm = nxm;
    qmlRegisterSingletonType<QQuickWebEngineProfile>("TurbineWeb", 1, 0, "NexusProfile", [](QQmlEngine *, QJSEngine *) -> QObject * {
        QQuickWebEngineProfile *p = profile();
        QJSEngine::setObjectOwnership(p, QJSEngine::CppOwnership);
        return p;
    });
}

// Settings, Accounts: Sign out of Nexus in Turbine. Every cookie and the cache go; the next page is signed out.
extern "C" int turbine_web_sign_out()
{
    QQuickWebEngineProfile *p = profile();
    if (!p)
        return 0;
    p->cookieStore()->deleteAllCookies();
    p->clearHttpCache();
    say(QStringLiteral("Signed out of Nexus in Turbine (cookies and cache cleared)"));
    return 1;
}

// One cookie from the player's own browser into this profile. host: the cookie's host with no leading dot, already
// checked to be nexusmods.com or under it (checked again here); host_only: a cookie for that host alone. expires:
// seconds since 1970, or 0 for a session cookie (kept anyway: ForcePersistentCookies). same_site: 0 not set, 1 lax,
// 2 strict, 3 none. Returns 1 when handed to the cookie store, 0 when refused.
extern "C" int turbine_web_set_cookie(const char *name, const char *value, const char *host, int host_only, const char *path,
                                      double expires, int secure, int http_only, int same_site)
{
    const QString h = QString::fromUtf8(host).toLower();
    if (!(h == QLatin1String("nexusmods.com") || h.endsWith(QLatin1String(".nexusmods.com"))))
        return 0;
    QQuickWebEngineProfile *p = profile();
    if (!p)
        return 0;
    QNetworkCookie c{QByteArray(name), QByteArray(value)};
    c.setDomain(host_only ? QString() : QStringLiteral(".") + h);
    c.setPath(path && *path ? QString::fromUtf8(path) : QStringLiteral("/"));
    c.setSecure(secure != 0);
    c.setHttpOnly(http_only != 0);
    if (expires > 0)
        c.setExpirationDate(QDateTime::fromSecsSinceEpoch(static_cast<qint64>(expires)));
    switch (same_site) {
    case 1:
        c.setSameSitePolicy(QNetworkCookie::SameSite::Lax);
        break;
    case 2:
        c.setSameSitePolicy(QNetworkCookie::SameSite::Strict);
        break;
    case 3:
        c.setSameSitePolicy(QNetworkCookie::SameSite::None);
        break;
    default:
        c.setSameSitePolicy(QNetworkCookie::SameSite::Default);
        break;
    }
    p->cookieStore()->setCookie(c, QUrl(QStringLiteral("https://") + h + QStringLiteral("/")));
    return 1;
}
