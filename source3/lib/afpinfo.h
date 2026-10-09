/*
 * AFP_AfpInfo packing/unpacking helpers
 *
 * Copyright (C) Ralph Boehme, 2019
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, see <http://www.gnu.org/licenses/>.
 */

#ifndef _AFPINFO_H_
#define _AFPINFO_H_

#include "replace.h"
#include <talloc.h>
#include "MacExtensions.h"

AfpInfo *afpinfo_new(TALLOC_CTX *ctx);
ssize_t afpinfo_pack(const AfpInfo *ai, char *buf);
AfpInfo *afpinfo_unpack(TALLOC_CTX *ctx, const void *data, bool validate);

#endif
