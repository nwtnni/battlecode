#[derive(Debug)]
pub struct Disjoint {
    size: i32,
    parent: Vec<i32>,
    rank: Vec<i32>,
}

impl Disjoint {

    pub fn new(size: i32) {
        let parent = (0..size).collect::<Vec<_>>();
        let rank = vec![0; size as usize];
    }

    pub fn find(&mut self, x: i32) -> i32 {
        let mut y = x as usize;
        let mut path = Vec::new();

        while self.parent[y] as usize != y {
            path.push(y);
            y = self.parent[y] as usize;
        }

        path.into_iter().for_each(|x| self.parent[x] = y as i32);
        self.parent[x as usize]
    }

    pub fn union(&mut self, x: i32, y: i32) {
        let x_root = self.find(x) as usize;
        let y_root = self.find(y) as usize;
        if x_root == y_root { return }

        if self.rank[x_root] < self.rank[y_root] {
            self.parent[x_root] = y_root as i32;
        } else if self.rank[x_root] > self.rank[y_root] {
            self.parent[y_root] = x_root as i32;
        } else {
            self.parent[y_root] = x_root as i32;
            self.rank[x_root] += 1;
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_basic() {
        let mut set = Disjoint::new(5);
        println!("{:?}", set);
        set.union(0, 4);
        println!("{:?}", set);
        set.union(2, 3);
        println!("{:?}", set);
        set.union(4, 2);
        println!("{:?}", set);
        set.find(3);
        println!("{:?}", set);
    }
}
