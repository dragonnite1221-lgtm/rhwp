import SwiftUI

/// 앱 전체 구조를 담당하는 최상위 뷰.
/// NavigationStack + 하단 툴바 구조.
/// 향후 탭 확장 시 TabView + 여러 DocumentView를 배치.
struct ContentView: View {
    @StateObject private var viewModel = DocumentViewModel()
    @State private var showFilePicker = false

    var body: some View {
        NavigationStack {
            DocumentView(viewModel: viewModel)
                .navigationBarHidden(true)
                .toolbarBackground(.visible, for: .bottomBar)
                .toolbarBackground(Color(UIColor.systemBackground), for: .bottomBar)
                .toolbar {
                    ToolbarItemGroup(placement: .bottomBar) {
                        Button {
                            showFilePicker = true
                        } label: {
                            Image(systemName: "folder")
                        }
                        .accessibilityLabel("문서 열기")
                        .accessibilityHint("파일앱에서 HWP 또는 HWPX 문서를 선택합니다")

                        Spacer()

                        if viewModel.pageCount > 0 {
                            Text("\(viewModel.currentPage + 1) / \(viewModel.pageCount)")
                                .font(.footnote.monospacedDigit())
                                .foregroundColor(.secondary)
                                .accessibilityLabel("\(viewModel.pageCount)쪽 중 \(viewModel.currentPage + 1)쪽")
                        }

                        Spacer()

                        // 우측 균형: 동일 크기 투명 플레이스홀더 (P1에서 공유/설정 배치 예정)
                        Button {} label: {
                            Image(systemName: "folder")
                        }
                        .hidden()
                        .accessibilityHidden(true)
                    }
                }
                .sheet(isPresented: $showFilePicker) {
                    DocumentPickerView(
                        onPick: { data, filename in
                            viewModel.loadDocument(data: data, filename: filename)
                        },
                        onError: { err in
                            viewModel.errorMessage = err.errorDescription
                        }
                    )
                }
                .onAppear {
                    // 번들 샘플이 있으면 자동 로드
                    if viewModel.document == nil {
                        viewModel.loadSampleFromBundle()
                    }
                }
        }
    }
}
