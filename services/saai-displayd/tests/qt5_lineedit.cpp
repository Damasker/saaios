// APP-04 host probe: focused QLineEdit against saai-displayd.
// Prints QT_LINEEDIT_TEXT= on every change. Not a panther field.
#include <QApplication>
#include <QLineEdit>
#include <QVBoxLayout>
#include <QWidget>
#include <cstdio>

int main(int argc, char **argv) {
    qputenv("QT_QPA_PLATFORM", "wayland");
    QApplication app(argc, argv);

    QLineEdit *edit = nullptr;
    QWidget window;
    if (qEnvironmentVariableIsSet("QT_LINEEDIT_COMPETE")) {
        // ADR-360: LocationBar/Filter class. A non-IM pane holds
        // focus (WebView/FolderView). The field is the top 40 px.
        window.resize(320, 200);
        window.setWindowFlags(Qt::FramelessWindowHint);
        auto *layout = new QVBoxLayout(&window);
        layout->setContentsMargins(0, 0, 0, 0);
        layout->setSpacing(0);
        edit = new QLineEdit;
        edit->setPlaceholderText(QStringLiteral("local field"));
        edit->setFixedHeight(40);
        auto *pane = new QWidget;
        pane->setFocusPolicy(Qt::StrongFocus);
        pane->setMinimumHeight(160);
        layout->addWidget(edit);
        layout->addWidget(pane, 1);
        pane->setFocus(Qt::OtherFocusReason);
        window.show();
    } else {
        edit = new QLineEdit;
        edit->setPlaceholderText(QStringLiteral("local field"));
        edit->resize(320, 40);
        edit->show();
        // ADR-359: a lone QLineEdit auto-focuses on Activated.
        if (!qEnvironmentVariableIsSet("QT_LINEEDIT_NO_SETFOCUS")) {
            edit->setFocus(Qt::OtherFocusReason);
        }
    }

    QObject::connect(edit, &QLineEdit::textChanged, [](const QString &text) {
        std::fprintf(stdout, "QT_LINEEDIT_TEXT=%s\n", text.toUtf8().constData());
        std::fflush(stdout);
    });
    return app.exec();
}
