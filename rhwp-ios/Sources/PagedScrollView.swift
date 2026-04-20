// PagedScrollView — UIScrollView 기반 다중 페이지 + 팬/줌
// 1단계: 기본 구조 (스크롤만, 줌은 2단계에서 추가)

import SwiftUI
import UIKit

/// SwiftUI에서 사용하는 UIScrollView 기반 문서 뷰
struct PagedScrollView: UIViewRepresentable {
    @ObservedObject var viewModel: DocumentViewModel

    func makeUIView(context: Context) -> PagedScrollUIView {
        let view = PagedScrollUIView()
        view.viewModel = viewModel
        view.reload()
        return view
    }

    func updateUIView(_ uiView: PagedScrollUIView, context: Context) {
        uiView.viewModel = viewModel
        // 문서 핸들 변경 또는 페이지 수 변경 시 재로드
        let currentDocId = viewModel.document.map { ObjectIdentifier($0) } ?? ObjectIdentifier(NSObject())
        if uiView.loadedDocumentId != currentDocId {
            uiView.reload()
        }
    }
}

/// UIScrollView 서브클래스 — 페이지 레이아웃, lazy 로드/언로드
class PagedScrollUIView: UIScrollView, UIScrollViewDelegate {
    weak var viewModel: DocumentViewModel?

    private let containerView = UIView()
    private var pageViews: [Int: PageCanvasUIView] = [:]
    private var pageFrames: [CGRect] = []
    private let pageSpacing: CGFloat = 12
    private let verticalPadding: CGFloat = 8

    var loadedDocumentId: ObjectIdentifier = ObjectIdentifier(NSObject())
    private var contentWidth: CGFloat = 0
    private var contentHeight: CGFloat = 0
    private var fitScale: CGFloat = 1.0
    /// 문서가 처음 로드된 후 최초 레이아웃에서 fitScale 적용을 위한 플래그
    private var pendingInitialFit = false

    override init(frame: CGRect) {
        super.init(frame: frame)
        setup()
    }

    required init?(coder: NSCoder) {
        super.init(coder: coder)
        setup()
    }

    private func setup() {
        delegate = self
        backgroundColor = UIColor { trait in
            // 라이트: 중간 회색, 다크: 더 어두운 회색 — 페이지(흰색/어두운)와 툴바(systemBackground) 양쪽과 구분
            trait.userInterfaceStyle == .dark
                ? UIColor(white: 0.12, alpha: 1.0)
                : UIColor(white: 0.82, alpha: 1.0)
        }
        showsHorizontalScrollIndicator = false
        showsVerticalScrollIndicator = true
        // 줌 설정
        minimumZoomScale = 1.0
        maximumZoomScale = 5.0
        bouncesZoom = true
        addSubview(containerView)
        containerView.backgroundColor = .clear

        // 더블탭 제스처
        let doubleTap = UITapGestureRecognizer(target: self, action: #selector(handleDoubleTap(_:)))
        doubleTap.numberOfTapsRequired = 2
        addGestureRecognizer(doubleTap)
    }

    // MARK: - 더블탭 줌

    @objc private func handleDoubleTap(_ gesture: UITapGestureRecognizer) {
        let doubleTapZoomScale: CGFloat = fitScale * 2.5
        if zoomScale > fitScale * 1.01 {
            // 현재 확대된 상태 → fit 으로 축소
            setZoomScale(fitScale, animated: true)
        } else {
            // fit 상태 → 탭한 위치 중심으로 2.5배 확대
            let tapPoint = gesture.location(in: containerView)
            let targetZoom = min(doubleTapZoomScale, maximumZoomScale)
            let rect = zoomRect(forScale: targetZoom, center: tapPoint)
            zoom(to: rect, animated: true)
        }
    }

    private func zoomRect(forScale scale: CGFloat, center: CGPoint) -> CGRect {
        let w = bounds.width / scale
        let h = bounds.height / scale
        return CGRect(x: center.x - w / 2, y: center.y - h / 2, width: w, height: h)
    }

    // MARK: - 줌 지원

    func viewForZooming(in scrollView: UIScrollView) -> UIView? {
        return containerView
    }

    /// fit 스케일 계산 (뷰포트 너비 / 문서 최대 너비)
    private func computeFitScale() -> CGFloat {
        guard contentWidth > 0, bounds.width > 0 else { return 1.0 }
        return min(1.0, bounds.width / contentWidth)
    }

    /// 레이아웃 변경 시 fitScale 갱신
    private func updateZoomBounds() {
        let fit = computeFitScale()
        fitScale = fit
        minimumZoomScale = fit
        maximumZoomScale = max(fit * 5.0, 5.0)
        if zoomScale < fit {
            zoomScale = fit
        }
    }

    // MARK: - 레이아웃 계산

    func reload() {
        guard let vm = viewModel, let doc = vm.document else {
            // 문서 없음: 클리어
            pageViews.values.forEach { $0.removeFromSuperview() }
            pageViews.removeAll()
            pageFrames.removeAll()
            contentSize = .zero
            return
        }

        loadedDocumentId = ObjectIdentifier(doc)

        // 페이지 frame 계산
        pageFrames.removeAll()
        var y: CGFloat = verticalPadding
        var maxWidth: CGFloat = 0

        for page in 0..<vm.pageCount {
            let size = vm.pageSize(at: page)
            let w = CGFloat(size.width)
            let h = CGFloat(size.height)
            pageFrames.append(CGRect(x: 0, y: y, width: w, height: h))
            y += h + pageSpacing
            if w > maxWidth { maxWidth = w }
        }
        y += verticalPadding - pageSpacing
        contentWidth = maxWidth
        contentHeight = y

        // 기존 페이지 뷰 제거
        pageViews.values.forEach { $0.removeFromSuperview() }
        pageViews.removeAll()

        // 가로 중앙 정렬 (페이지별 x 오프셋 재조정)
        for i in 0..<pageFrames.count {
            let originalFrame = pageFrames[i]
            let x = (maxWidth - originalFrame.width) / 2
            pageFrames[i] = CGRect(x: x, y: originalFrame.origin.y,
                                   width: originalFrame.width, height: originalFrame.height)
        }

        containerView.frame = CGRect(x: 0, y: 0, width: contentWidth, height: contentHeight)
        contentSize = CGSize(width: contentWidth, height: contentHeight)

        // 초기 스크롤 위치 리셋
        contentOffset = .zero

        // bounds가 아직 확정되지 않았을 수 있으므로 플래그만 설정,
        // 실제 fitScale 적용은 layoutSubviews에서 수행
        pendingInitialFit = true
        setNeedsLayout()
    }

    /// 줌된 콘텐츠를 가로 중앙 정렬 (뷰포트보다 작을 때)
    private func adjustContentInset() {
        let scaledWidth = contentSize.width * zoomScale
        let horizontalInset = max(0, (bounds.width - scaledWidth) / 2)
        contentInset = UIEdgeInsets(top: 0, left: horizontalInset, bottom: 0, right: horizontalInset)
    }

    // MARK: - Lazy 로드/언로드

    override func layoutSubviews() {
        super.layoutSubviews()
        updateZoomBounds()
        // bounds가 확정된 후 처음 문서 로드 시 fit 스케일로 시작
        if pendingInitialFit, bounds.width > 0, contentWidth > 0 {
            zoomScale = fitScale
            contentOffset = CGPoint(x: 0, y: -contentInset.top)
            pendingInitialFit = false
        }
        adjustContentInset()
        layoutPagesIfNeeded()
    }

    func scrollViewDidScroll(_ scrollView: UIScrollView) {
        layoutPagesIfNeeded()
    }

    func scrollViewDidZoom(_ scrollView: UIScrollView) {
        adjustContentInset()
    }

    func scrollViewDidEndZooming(_ scrollView: UIScrollView, with view: UIView?, atScale scale: CGFloat) {
        // 줌 종료 후 Core Graphics 재렌더링으로 선명도 확보
        updatePageContentScales()
        layoutPagesIfNeeded()
    }

    /// 현재 줌 스케일에 맞춰 페이지 뷰의 contentScaleFactor 조정
    /// (Retina × zoom 만큼 해상도 높여서 Core Graphics가 재그리기)
    private func updatePageContentScales() {
        let baseScale = UIScreen.main.scale
        let targetScale = baseScale * zoomScale
        for (_, pageView) in pageViews {
            if pageView.contentScaleFactor != targetScale {
                pageView.contentScaleFactor = targetScale
                pageView.layer.contentsScale = targetScale
                pageView.setNeedsDisplay()
            }
        }
    }

    private func layoutPagesIfNeeded() {
        guard !pageFrames.isEmpty, let vm = viewModel else { return }

        // 현재 가시 영역을 containerView 좌표계(원본)로 변환 (줌 스케일 반영)
        let zoom = max(zoomScale, 0.0001)
        let visibleInContainer = CGRect(
            x: contentOffset.x / zoom,
            y: contentOffset.y / zoom,
            width: bounds.width / zoom,
            height: bounds.height / zoom
        )
        let loadMargin: CGFloat = visibleInContainer.height * 0.5
        let expandedRect = visibleInContainer.insetBy(dx: 0, dy: -loadMargin)

        // 로드해야 할 페이지 + 언로드할 페이지 식별
        var shouldLoad: Set<Int> = []
        for (i, frame) in pageFrames.enumerated() {
            if frame.intersects(expandedRect) {
                shouldLoad.insert(i)
            }
        }

        // 언로드
        let currentlyLoaded = Set(pageViews.keys)
        let toUnload = currentlyLoaded.subtracting(shouldLoad)
        for page in toUnload {
            if let view = pageViews.removeValue(forKey: page) {
                view.removeFromSuperview()
            }
            vm.unloadPage(page)
        }

        // 로드
        let toLoad = shouldLoad.subtracting(currentlyLoaded)
        let currentContentScale = UIScreen.main.scale * zoomScale
        for page in toLoad {
            vm.loadPage(page)
            let canvas = PageCanvasUIView()
            canvas.frame = pageFrames[page]
            canvas.configure(
                tree: vm.pageTrees[page],
                pageHeight: Double(pageFrames[page].height),
                document: vm.document
            )
            canvas.contentScaleFactor = currentContentScale
            canvas.layer.contentsScale = currentContentScale
            canvas.layer.shadowColor = UIColor.black.cgColor
            canvas.layer.shadowOpacity = 0.08
            canvas.layer.shadowOffset = CGSize(width: 0, height: 1)
            canvas.layer.shadowRadius = 3
            containerView.addSubview(canvas)
            pageViews[page] = canvas
        }

        // 현재 페이지 추적 (가시 영역 중심을 원본 좌표로 변환)
        let centerY = (contentOffset.y + bounds.height / 2) / zoom
        for (i, frame) in pageFrames.enumerated() {
            if frame.minY <= centerY && centerY <= frame.maxY {
                if vm.currentPage != i {
                    DispatchQueue.main.async {
                        vm.currentPage = i
                    }
                }
                break
            }
        }
    }
}
