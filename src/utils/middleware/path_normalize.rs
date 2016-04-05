// Copyright (C) 2016  Max Planck Institute for Human Development
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published
// by the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

use iron::{BeforeMiddleware, IronError, IronResult, Request};

pub struct PathNormalizer;
impl BeforeMiddleware for PathNormalizer
{
    fn before(&self, req: &mut Request) -> IronResult<()>
    {
        if req.url.path.len() > 1 {
            let first_elem_empty = &*req.url.path.first().unwrap() == "";
            if first_elem_empty {
                req.url.path.remove(0);
            }
            let last_elem = req.url.path.pop().unwrap();
            if &*last_elem != "" {
                req.url.path.push(last_elem);
            }
        }
        Ok(())
    }
    fn catch(&self, _: &mut Request, _: IronError) -> IronResult<()>
    {
        Ok(())
    }
}
