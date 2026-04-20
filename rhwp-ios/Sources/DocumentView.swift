import SwiftUI

/// 단일 HWP 문서를 렌더링하는 뷰.
/// 향후 탭 확장 시 탭 1개 = DocumentView 1개로 대응.
struct DocumentView: View {
    @ObservedObject var viewModel: DocumentViewModel

    var body: some View {
        VStack(spacing: 0) {
            // 상단 정보 바
            headerBar

            // 문서 렌더링 영역
            if viewModel.isLoading {
                Spacer()
                ProgressView("로딩 중...")
                Spacer()
            } else if viewModel.pageCount > 0 {
                pageScrollView
            } else if let error = viewModel.errorMessage {
                errorView(error)
            } else {
                Spacer()
                Text("문서를 열어주세요.")
                    .foregroundColor(.secondary)
                Spacer()
            }
        }
    }

    // MARK: - 상단 바

    private var headerBar: some View {
        HStack {
            Text("알한글")
                .font(.headline)
                .fontWeight(.bold)
            if !viewModel.filename.isEmpty {
                Text("— \(viewModel.filename)")
                    .font(.caption)
                    .foregroundColor(.secondary)
                    .lineLimit(1)
            }
            Spacer()
            if viewModel.pageCount > 0 {
                Text("\(viewModel.currentPage + 1)/\(viewModel.pageCount)쪽")
                    .font(.caption)
                    .foregroundColor(.secondary)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 10)
        .background(Color(UIColor.systemBackground))
    }

    // MARK: - 다중 페이지 스크롤 (뷰포트 너비 맞춤)

    private var pageScrollView: some View {
        GeometryReader { geo in
            let viewportWidth = geo.size.width - 16   // 좌우 여백 8씩
            ScrollView {
                LazyVStack(spacing: 12) {
                    ForEach(0..<viewModel.pageCount, id: \.self) { page in
                        let size = viewModel.pageSize(at: page)
                        let scale = size.width > 0 ? min(1.0, viewportWidth / size.width) : 1.0
                        HStack {
                            Spacer(minLength: 0)
                            PageCanvasView(
                                renderTree: viewModel.pageTrees[page],
                                pageHeight: size.height,
                                document: viewModel.document
                            )
                            .frame(width: CGFloat(size.width), height: CGFloat(size.height))
                            .background(Color.white)
                            .shadow(color: .black.opacity(0.08), radius: 3, x: 0, y: 1)
                            .scaleEffect(scale, anchor: .center)
                            .frame(
                                width: CGFloat(size.width) * scale,
                                height: CGFloat(size.height) * scale
                            )
                            Spacer(minLength: 0)
                        }
                        .onAppear {
                            viewModel.loadPage(page)
                            viewModel.currentPage = page
                        }
                        .onDisappear {
                            viewModel.unloadPage(page)
                        }
                    }
                }
                .padding(.vertical, 8)
                .frame(maxWidth: .infinity)
            }
            .background(Color(UIColor.systemGroupedBackground))
        }
    }

    // MARK: - 에러

    private func errorView(_ error: String) -> some View {
        VStack {
            Spacer()
            VStack(spacing: 8) {
                Image(systemName: "exclamationmark.triangle")
                    .font(.largeTitle)
                    .foregroundColor(.orange)
                Text(error)
                    .foregroundColor(.secondary)
                    .multilineTextAlignment(.center)
            }
            .padding()
            Spacer()
        }
    }
}
