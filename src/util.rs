use std::array;

pub trait IteratorArrayExt: Iterator
where
    Self: Sized,
{
    fn collect_to_array_padded<const N: usize>(
        self,
        mut default: impl FnMut() -> Self::Item,
    ) -> [Self::Item; N] {
        let mut it = self.fuse();
        array::from_fn(|_| it.next().unwrap_or_else(&mut default))
    }
}

impl<T> IteratorArrayExt for T where T: Iterator {}
