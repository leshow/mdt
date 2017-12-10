fn count<T: PartialOrd>(arr: &[T], x: &T) -> Option<usize> {
    let f = first(&arr, &x, 0, arr.len())?;
    let l = last(&arr, &x, f, arr.len())?;
    Some(l - f + 1)
}
fn first<T: PartialOrd>(arr: &[T], x: &T, l: usize, r: usize) -> Option<usize> {
    if l <= r {
        let mid = l + r >> 1;
        if (mid == 0 || arr[mid - 1] < *x) && arr[mid] == *x {
            return Some(mid);
        } else if arr[mid] < *x {
            return first(arr, x, mid + 1, r);
        } else {
            return first(arr, x, l, mid - 1);
        }
    }
    None
}
fn last<T: PartialOrd>(arr: &[T], x: &T, l: usize, r: usize) -> Option<usize> {
    if l <= r {
        let mid: usize = l + r >> 1;
        if (mid == arr.len() - 1 || arr[mid + 1] > *x) && arr[mid] == *x {
            return Some(mid);
        } else if arr[mid] > *x {
            return last(arr, x, l, mid - 1);
        } else {
            return last(arr, x, mid + 1, r);
        }
    }
    None
}

#[cfg(test)]
#[test]
fn test_count() {
    let a = vec![1, 2, 3, 3, 3, 3, 3, 3, 3, 4, 7];
    let mut b = vec![1, 2, 3, 9, 3, 4, 28, 7, 9];
    b.sort();
    assert_eq!(count(&a[..], &3), Some(7));
    assert_eq!(count(&b[..], &1), Some(1));
}
