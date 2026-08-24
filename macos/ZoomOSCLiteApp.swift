import SwiftUI
import AppKit
import Darwin

@_silgen_name("zoomosc_execute")
private func zoomoscExecute(_ command: UnsafePointer<CChar>) -> Int32

@_silgen_name("zoomosc_accessibility_trusted")
private func zoomoscAccessibilityTrusted() -> Bool

@_silgen_name("zoomosc_request_accessibility")
private func zoomoscRequestAccessibility() -> Bool

@MainActor
final class ServerController: ObservableObject {
    @Published var portText: String
    @Published var networkMode: String
    @Published var status = "Parado"
    @Published var lastMessage = ""
    @Published var isRunning = false

    private var socketFD: Int32 = -1
    private var socketSource: DispatchSourceRead?
    private let defaults = UserDefaults.standard

    init() {
        let savedPort = defaults.integer(forKey: "oscPort")
        portText = String(savedPort == 0 ? 9000 : savedPort)
        networkMode = defaults.string(forKey: "networkMode") ?? "local"
    }

    var bindAddress: String {
        networkMode == "lan" ? "0.0.0.0" : "127.0.0.1"
    }

    var endpoint: String {
        "osc.udp://\(bindAddress):\(portText)"
    }

    var portIsValid: Bool {
        guard let port = Int(portText) else { return false }
        return (1...65535).contains(port)
    }

    func start() {
        guard portIsValid else {
            status = "Porta inválida"
            return
        }

        stop()
        defaults.set(Int(portText), forKey: "oscPort")
        defaults.set(networkMode, forKey: "networkMode")

        let port = UInt16(portText)!
        let fd = Darwin.socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP)
        guard fd >= 0 else {
            status = "Erro ao criar socket UDP"
            return
        }
        var reuse: Int32 = 1
        setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &reuse, socklen_t(MemoryLayout.size(ofValue: reuse)))

        var address = sockaddr_in()
        address.sin_len = UInt8(MemoryLayout<sockaddr_in>.size)
        address.sin_family = sa_family_t(AF_INET)
        address.sin_port = in_port_t(port.bigEndian)
        address.sin_addr = in_addr(s_addr: inet_addr(bindAddress))
        let bindResult = withUnsafePointer(to: &address) {
            $0.withMemoryRebound(to: sockaddr.self, capacity: 1) {
                Darwin.bind(fd, $0, socklen_t(MemoryLayout<sockaddr_in>.size))
            }
        }
        guard bindResult == 0 else {
            Darwin.close(fd)
            status = "Não foi possível abrir \(endpoint)"
            return
        }

        let source = DispatchSource.makeReadSource(
            fileDescriptor: fd,
            queue: DispatchQueue(label: "pt.joaocarvalho.zoomosc-lite.udp")
        )
        source.setEventHandler { [weak self] in
            var buffer = [UInt8](repeating: 0, count: 65_535)
            let count = Darwin.recv(fd, &buffer, buffer.count, 0)
            guard count > 0, buffer[0] == Character("/").asciiValue,
                  let end = buffer[..<Int(count)].firstIndex(of: 0),
                  let command = String(bytes: buffer[..<end], encoding: .utf8) else { return }
            let result = command.withCString { zoomoscExecute($0) }
            Task { @MainActor in
                self?.lastMessage = result == 0
                    ? "Executado: \(command)"
                    : "Erro ao executar: \(command)"
            }
        }
        source.setCancelHandler { Darwin.close(fd) }
        socketFD = fd
        socketSource = source
        source.resume()
        isRunning = true
        status = "Ativo em \(endpoint)"

        if !zoomoscAccessibilityTrusted() {
            _ = zoomoscRequestAccessibility()
            lastMessage = "Autoriza ZoomOSC Lite em Acessibilidade"
        }
    }

    func stop() {
        socketSource?.cancel()
        socketSource = nil
        socketFD = -1
        isRunning = false
        status = "Parado"
    }

    func openAccessibilitySettings() {
        _ = zoomoscRequestAccessibility()
        if let url = URL(string: "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility") {
            NSWorkspace.shared.open(url)
        }
    }
}

struct ContentView: View {
    @ObservedObject var server: ServerController

    var body: some View {
        VStack(alignment: .leading, spacing: 20) {
            HStack(spacing: 12) {
                Image(nsImage: NSApp.applicationIconImage)
                    .resizable()
                    .frame(width: 44, height: 44)
                VStack(alignment: .leading, spacing: 2) {
                    Text("ZoomOSC Lite")
                        .font(.title2.bold())
                    Text("Controlo OSC para Zoom Workplace")
                        .foregroundStyle(.secondary)
                }
            }

            GroupBox("Servidor OSC") {
                Grid(alignment: .leading, horizontalSpacing: 16, verticalSpacing: 14) {
                    GridRow {
                        Text("Acesso")
                        Picker("Acesso", selection: $server.networkMode) {
                            Text("Apenas este Mac").tag("local")
                            Text("Rede local").tag("lan")
                        }
                        .labelsHidden()
                        .pickerStyle(.segmented)
                    }
                    GridRow {
                        Text("Porta UDP")
                        TextField("9000", text: $server.portText)
                            .textFieldStyle(.roundedBorder)
                            .frame(width: 110)
                    }
                    GridRow {
                        Text("Endereço")
                        Text(server.endpoint)
                            .font(.system(.body, design: .monospaced))
                            .foregroundStyle(.secondary)
                    }
                    GridRow {
                        Text("Estado")
                        HStack(spacing: 7) {
                            Circle()
                                .fill(server.isRunning ? Color.green : Color.orange)
                                .frame(width: 9, height: 9)
                            Text(server.status)
                        }
                    }
                }
                .padding(10)
            }

            HStack {
                Button("Acessibilidade…") {
                    server.openAccessibilitySettings()
                }
                Spacer()
                Button(server.isRunning ? "Aplicar e reiniciar" : "Iniciar servidor") {
                    server.start()
                }
                .keyboardShortcut(.defaultAction)
                .disabled(!server.portIsValid)
            }

            GroupBox("Comandos principais") {
                VStack(spacing: 2) {
                    command("/zoom/audio/mute", "Desativar microfone")
                    command("/zoom/audio/unmute", "Ativar microfone")
                    command("/zoom/video/on", "Ligar vídeo")
                    command("/zoom/video/off", "Desligar vídeo")
                    command("/zoom/share/camera/start", "Partilhar segunda câmara")
                    command("/zoom/share/stop", "Parar partilha")
                    Divider().padding(.vertical, 4)
                    command("/zoom/audio/profile/noise-removal", "Remoção de ruído")
                    command("/zoom/audio/profile/isolation", "Isolamento personalizado")
                    command("/zoom/audio/profile/original", "Som original para músicos")
                    command("/zoom/audio/profile/live-performance", "Performance ao vivo")
                }
                .padding(8)
            }

            if !server.lastMessage.isEmpty {
                Text(server.lastMessage)
                    .font(.caption.monospaced())
                    .foregroundStyle(.secondary)
                    .lineLimit(3)
            }
        }
        .padding(24)
        .frame(width: 570)
        .onAppear { server.start() }
    }

    @ViewBuilder
    private func command(_ address: String, _ description: String) -> some View {
        Button {
            let pasteboard = NSPasteboard.general
            pasteboard.clearContents()
            pasteboard.setString(address, forType: .string)
            server.lastMessage = "Copiado: \(address)"
        } label: {
            HStack(spacing: 12) {
                Text(address)
                    .font(.system(.callout, design: .monospaced))
                    .foregroundStyle(.primary)
                Spacer()
                Text(description)
                    .foregroundStyle(.secondary)
                Image(systemName: "doc.on.doc")
                    .foregroundStyle(.blue)
            }
            .padding(.horizontal, 8)
            .padding(.vertical, 6)
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .help("Copiar \(address)")
    }
}

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
    weak var server: ServerController?

    func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
        true
    }

    func applicationWillTerminate(_ notification: Notification) {
        server?.stop()
    }
}

@main
struct ZoomOSCLiteApp: App {
    @NSApplicationDelegateAdaptor(AppDelegate.self) private var appDelegate
    @StateObject private var server: ServerController

    init() {
        _server = StateObject(wrappedValue: ServerController())
    }

    var body: some Scene {
        WindowGroup {
            ContentView(server: server)
                .onAppear {
                    appDelegate.server = server
                }
        }
        .windowResizability(.contentSize)
        .commands {
            CommandGroup(replacing: .newItem) {}
        }
    }
}
