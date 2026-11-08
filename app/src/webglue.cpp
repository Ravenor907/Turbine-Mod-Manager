// Nexus inside Turbine: the parts of Qt WebEngine the Rust side can't reach through cxx-qt-lib.
// - turbine_web_prepare: before the application object exists, the nxm scheme is registered (Qt ignores it later)
//   and QtWebEngineQuick::initialize() runs. Both are only set up here; no web page is loaded.
// - turbine_web_profile: remembers where the browser keeps its files and registers the one profile Turbine uses as the
//   QML singleton TurbineWeb.NexusProfile. The profile itself is made the first time something uses it (opening the
//   browser, signing out), so Chromium never starts in a run that doesn't open Nexus.
// - NxmHandler: a Mod Manager Download link the page navigates to stops here (the page stays where it is). Only a
//   request whose initiator is nexusmods.com over https (or the build's stand-in) goes on to Rust (browse.rs), which
//   checks it again: QML's guard must have let the main frame go there while it was on Nexus, the link must carry its
//   key, expiry and account number, the player must have pressed something in the last 30 seconds, and the link must
//   be new. Anything else is refused here and only its origin is logged.
// - turbine_web_sign_out, turbine_web_set_cookie: Settings' sign out, and the sign-in the browser helper brings back
//   from the player's own browser (nexusmods.com cookies only, checked in Rust first and again here).
// - The cookie store is watched for Nexus's sign-in cookie (names only), so Turbine knows whether the page is signed in.
// - The profile's folders are the player's alone (0700): they hold the sign-in.
// Cookie values never go to the log: only names and counts do.
// No class here has Q_OBJECT, so nothing in this file needs moc.

#include <QtCore/QByteArray>
#include <QtCore/QDateTime>
#include <QtCore/QDir>
#include <QtCore/QFile>
#include <QtCore/QSet>
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
using NxmSink = void (*)(const char *url, const char *initiator);
using SignedSink = void (*)(int on);

LogSink s_log = nullptr;
NxmSink s_nxm = nullptr;
SignedSink s_signed = nullptr;
QString s_data;
QString s_cache;
QString s_testOrigin; // "http://127.0.0.1:<port>" in the build's test runs only, else empty
QSet<QString> s_signinCookies;
QPointer<QQuickWebEngineProfile> s_profile;

void say(const QString &text)
{
    if (s_log) {
        const QByteArray b = text.toUtf8();
        s_log(b.constData());
    }
}

// Is this origin ("https://www.nexusmods.com", "null", "http://127.0.0.1:47800") Nexus's own site over https, or the
// build's stand-in? browse.rs's initiator_ok says the same.
bool initiatorOk(const QUrl &origin)
{
    if (!s_testOrigin.isEmpty() && origin.toString(QUrl::RemovePath | QUrl::StripTrailingSlash).toLower() == s_testOrigin)
        return true;
    const QString host = origin.host().toLower();
    return origin.scheme() == QLatin1String("https") && (host == QLatin1String("nexusmods.com") || host.endsWith(QLatin1String(".nexusmods.com")));
}

// A Mod Manager Download link (nxm://...) the page navigated to. The request fails on purpose, so the page stays on
// the file page; the link's text goes to Rust only when Nexus itself started the request.
class NxmHandler : public QWebEngineUrlSchemeHandler
{
public:
    explicit NxmHandler(QObject *parent) : QWebEngineUrlSchemeHandler(parent) { }
    void requestStarted(QWebEngineUrlRequestJob *job) override
    {
        const QByteArray url = job->requestUrl().toString(QUrl::FullyEncoded).toUtf8();
        const QUrl origin = job->initiator();
        job->fail(QWebEngineUrlRequestJob::RequestAborted);
        const QString from = origin.isEmpty() ? QStringLiteral("nowhere") : origin.toString(QUrl::RemovePath | QUrl::RemoveQuery | QUrl::RemoveFragment | QUrl::RemoveUserInfo);
        if (!initiatorOk(origin)) {
            say(QStringLiteral("Mod Manager Download link refused: it came from %1, not nexusmods.com").arg(from));
            return;
        }
        say(QStringLiteral("Mod Manager Download link from %1: initiator checked").arg(from));
        const QByteArray o = from.toUtf8();
        if (s_nxm && url.size() <= 4096)
            s_nxm(url.constData(), o.constData());
    }
};

// Nexus's sign-in cookies (by name; values are never read): any one of them on nexusmods.com means signed in.
bool signinCookie(const QNetworkCookie &c)
{
    static const QSet<QString> names = { QStringLiteral("nexusmods_session"), QStringLiteral("nexusmods_session_refresh"), QStringLiteral("sid_develop"),
                                         QStringLiteral("member_id") };
    QString d = c.domain().toLower();
    if (d.startsWith(QLatin1Char('.')))
        d = d.mid(1);
    const bool nexus = d == QLatin1String("nexusmods.com") || d.endsWith(QLatin1String(".nexusmods.com"));
    return nexus && names.contains(QString::fromUtf8(c.name()));
}

void signinChanged()
{
    if (s_signed)
        s_signed(s_signinCookies.isEmpty() ? 0 : 1);
}

// A folder only the player can open (the profile holds the sign-in).
void ownerOnly(const QString &dir)
{
    QDir().mkpath(dir);
    QFile::setPermissions(dir, QFileDevice::ReadOwner | QFileDevice::WriteOwner | QFileDevice::ExeOwner);
}

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
        ownerOnly(s_data);
        p->setPersistentStoragePath(s_data);
    }
    if (!s_cache.isEmpty()) {
        ownerOnly(s_cache);
        p->setCachePath(s_cache);
    }
    p->setHttpCacheType(QQuickWebEngineProfile::DiskHttpCache);
    p->setPersistentCookiesPolicy(QQuickWebEngineProfile::ForcePersistentCookies);
    p->installUrlSchemeHandler(QByteArrayLiteral("nxm"), new NxmHandler(p));
    QWebEngineCookieStore *store = p->cookieStore();
    QObject::connect(store, &QWebEngineCookieStore::cookieAdded, p, [](const QNetworkCookie &c) {
        if (signinCookie(c)) {
            s_signinCookies.insert(QString::fromUtf8(c.name()) + QLatin1Char('@') + c.domain());
            signinChanged();
        }
    });
    QObject::connect(store, &QWebEngineCookieStore::cookieRemoved, p, [](const QNetworkCookie &c) {
        if (signinCookie(c)) {
            s_signinCookies.remove(QString::fromUtf8(c.name()) + QLatin1Char('@') + c.domain());
            signinChanged();
        }
    });
    store->loadAllCookies();
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
extern "C" void turbine_web_profile(const char *data_dir, const char *cache_dir, const char *test_origin, NxmSink nxm, SignedSink sign)
{
    s_data = QString::fromUtf8(data_dir);
    s_cache = QString::fromUtf8(cache_dir);
    s_testOrigin = QString::fromUtf8(test_origin).toLower();
    s_nxm = nxm;
    s_signed = sign;
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
    s_signinCookies.clear();
    signinChanged();
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
