import QtQuick 2.15
import org.kde.kirigami 2.20 as Kirigami

Kirigami.ApplicationWindow {
    id: root
    title: "SaaiOS Kirigami Demo"
    width: 1080
    height: 2400

    pageStack.initialPage: Kirigami.Page {
        title: "SaaiOS"

        Kirigami.Heading {
            id: heading
            anchors.centerIn: parent
            text: "Touch to test"
        }

        MouseArea {
            anchors.fill: parent
            onClicked: heading.text = "Touched!"
        }
    }
}
