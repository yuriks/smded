use egui::Id;
use std::pin::Pin;
use std::sync::Arc;
use std::task;
use std::task::{Poll, Waker};

pub struct EguiWaker(egui::Context);

impl EguiWaker {
    pub fn for_context(ctx: &egui::Context) -> Waker {
        if let Some(egui_waker) = ctx.data(|data| data.get_temp::<Arc<EguiWaker>>(Id::NULL)) {
            Waker::from(egui_waker)
        } else {
            let egui_waker = Self::new(ctx.clone());
            ctx.data_mut(|data| {
                data.insert_temp(Id::NULL, egui_waker.clone());
            });
            Waker::from(egui_waker)
        }
    }

    fn new(ctx: egui::Context) -> Arc<EguiWaker> {
        Arc::new(EguiWaker(ctx))
    }
}
impl task::Wake for EguiWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }

    fn wake_by_ref(self: &Arc<Self>) {
        self.0.request_repaint();
    }
}

pub struct Promise<F: Future> {
    waker: Waker,
    future: Option<F>,
    last_result: Option<F::Output>,
}

pub type LocalBoxFuture<T> = Pin<Box<dyn Future<Output = T>>>;

impl<F> Promise<F>
where
    F: Future + Unpin,
{
    pub fn new(waker: Waker) -> Self {
        Self {
            waker,
            future: None,
            last_result: None,
        }
    }

    pub fn launched(waker: Waker, future: F) -> Self {
        let mut slf = Self::new(waker);
        slf.launch(future);
        slf
    }

    pub fn launch(&mut self, future: F) {
        self.future = Some(future);
    }

    pub fn is_pending(&self) -> bool {
        self.future.is_some()
    }

    pub fn set_response(&mut self, response: F::Output) {
        self.last_result = Some(response);
    }

    fn poll_future(&mut self) {
        if let Some(future) = &mut self.future {
            let mut cx = task::Context::from_waker(&self.waker);
            if let Poll::Ready(res) = Pin::new(future).poll(&mut cx) {
                self.future = None;
                self.last_result = Some(res);
            }
        }
    }

    pub fn response(&mut self) -> Option<&F::Output> {
        self.poll_future();
        self.last_result.as_ref()
    }

    pub fn take_response(&mut self) -> Option<F::Output> {
        self.poll_future();
        self.last_result.take()
    }
}
