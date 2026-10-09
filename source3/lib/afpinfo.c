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

#include "replace.h"
#include "afpinfo.h"
#include "adouble.h"

/**
 * Allocate and initialize an AfpInfo struct
 **/
AfpInfo *afpinfo_new(TALLOC_CTX *ctx)
{
	AfpInfo *ai = talloc(ctx, AfpInfo);
	if (ai == NULL) {
		return NULL;
	}
	*ai = (AfpInfo){
		.afpi_Signature = AFP_Signature,
		.afpi_Version = AFP_Version,
		.afpi_BackupTime = AD_DATE_START,
	};
	return ai;
}

/**
 * Pack an AfpInfo struct into a buffer
 *
 * Buffer size must be at least AFP_INFO_SIZE
 * Returns size of packed buffer
 **/
ssize_t afpinfo_pack(const AfpInfo *ai, char *buf)
{
	memset(buf, 0, AFP_INFO_SIZE);

	RSIVAL(buf, 0, ai->afpi_Signature);
	RSIVAL(buf, 4, ai->afpi_Version);
	RSIVAL(buf, 12, ai->afpi_BackupTime);
	memcpy(buf + 16, ai->afpi_FinderInfo, sizeof(ai->afpi_FinderInfo));

	return AFP_INFO_SIZE;
}

/**
 * Unpack a buffer into a AfpInfo structure
 *
 * Buffer size must be at least AFP_INFO_SIZE
 * Returns allocated AfpInfo struct
 **/
AfpInfo *afpinfo_unpack(TALLOC_CTX *ctx, const void *data, bool validate)
{
	AfpInfo *ai = talloc_zero(ctx, AfpInfo);
	if (ai == NULL) {
		return NULL;
	}

	ai->afpi_Signature = RIVAL(data, 0);
	ai->afpi_Version = RIVAL(data, 4);
	ai->afpi_BackupTime = RIVAL(data, 12);
	memcpy(ai->afpi_FinderInfo, (const char *)data + 16,
	       sizeof(ai->afpi_FinderInfo));

	if (validate) {
		if (ai->afpi_Signature != AFP_Signature
		    || ai->afpi_Version != AFP_Version)
		{
			DEBUG(1, ("Bad AfpInfo signature or version\n"));
			TALLOC_FREE(ai);
		}
	} else {
		ai->afpi_Signature = AFP_Signature;
		ai->afpi_Version = AFP_Version;
	}

	return ai;
}
