; ModuleID = 'ct.c'
source_filename = "ct.c"
target datalayout = "e-m:o-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-apple-macosx10.14.0"

%struct.PartiallySecret = type { i32, i32 }
%struct.Parent = type { i32, %struct.Child*, %struct.Child* }
%struct.Child = type { i32, %struct.Parent* }
%struct.StructWithRelatedFields = type { i32, i32, i32 }

@__const.two_ct_violations.z = private unnamed_addr constant [3 x i32] [i32 0, i32 2, i32 300], align 4

; Function Attrs: norecurse nounwind readnone ssp uwtable
define i32 @ct_simple(i32) local_unnamed_addr #0 {
  %2 = add nsw i32 %0, 3
  ret i32 %2
}

; Function Attrs: nounwind ssp uwtable
define i32 @ct_simple2(i32, i32) local_unnamed_addr #1 {
  %3 = alloca i32, align 4
  %4 = bitcast i32* %3 to i8*
  call void @llvm.lifetime.start.p0i8(i64 4, i8* nonnull %4)
  store volatile i32 2, i32* %3, align 4, !tbaa !3
  %5 = load volatile i32, i32* %3, align 4, !tbaa !3
  %6 = icmp sgt i32 %5, 3
  br i1 %6, label %7, label %9

7:                                                ; preds = %2
  %8 = mul nsw i32 %0, 5
  br label %11

9:                                                ; preds = %2
  %10 = sdiv i32 %1, 99
  br label %11

11:                                               ; preds = %9, %7
  %12 = phi i32 [ %8, %7 ], [ %10, %9 ]
  call void @llvm.lifetime.end.p0i8(i64 4, i8* nonnull %4)
  ret i32 %12
}

; Function Attrs: argmemonly nounwind
declare void @llvm.lifetime.start.p0i8(i64 immarg, i8* nocapture) #2

; Function Attrs: argmemonly nounwind
declare void @llvm.lifetime.end.p0i8(i64 immarg, i8* nocapture) #2

; Function Attrs: norecurse nounwind readnone ssp uwtable
define i32 @notct_branch(i32) local_unnamed_addr #0 {
  %2 = icmp sgt i32 %0, 10
  br i1 %2, label %3, label %6

3:                                                ; preds = %1
  %4 = urem i32 %0, 200
  %5 = mul nuw nsw i32 %4, 3
  br label %8

6:                                                ; preds = %1
  %7 = add nsw i32 %0, 10
  br label %8

8:                                                ; preds = %6, %3
  %9 = phi i32 [ %5, %3 ], [ %7, %6 ]
  ret i32 %9
}

; Function Attrs: nounwind ssp uwtable
define i32 @notct_mem(i32) local_unnamed_addr #1 {
  %2 = alloca [3 x i32], align 4
  %3 = bitcast [3 x i32]* %2 to i8*
  call void @llvm.lifetime.start.p0i8(i64 12, i8* nonnull %3) #6
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* nonnull align 4 %3, i8* align 4 bitcast ([3 x i32]* @__const.two_ct_violations.z to i8*), i64 12, i1 true)
  %4 = srem i32 %0, 3
  %5 = sext i32 %4 to i64
  %6 = getelementptr inbounds [3 x i32], [3 x i32]* %2, i64 0, i64 %5
  %7 = load volatile i32, i32* %6, align 4, !tbaa !3
  call void @llvm.lifetime.end.p0i8(i64 12, i8* nonnull %3) #6
  ret i32 %7
}

; Function Attrs: argmemonly nounwind
declare void @llvm.memcpy.p0i8.p0i8.i64(i8* nocapture writeonly, i8* nocapture readonly, i64, i1 immarg) #2

; Function Attrs: nounwind ssp uwtable
define i32 @notct_truepath(i32, i32, i32) local_unnamed_addr #1 {
  %4 = alloca [3 x i32], align 4
  %5 = bitcast [3 x i32]* %4 to i8*
  call void @llvm.lifetime.start.p0i8(i64 12, i8* nonnull %5) #6
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* nonnull align 4 %5, i8* align 4 bitcast ([3 x i32]* @__const.two_ct_violations.z to i8*), i64 12, i1 true)
  %6 = getelementptr inbounds [3 x i32], [3 x i32]* %4, i64 0, i64 2
  store volatile i32 %1, i32* %6, align 4, !tbaa !3
  %7 = icmp sgt i32 %2, 3
  br i1 %7, label %8, label %11

8:                                                ; preds = %3
  %9 = srem i32 %0, 3
  %10 = sext i32 %9 to i64
  br label %11

11:                                               ; preds = %3, %8
  %12 = phi i64 [ %10, %8 ], [ 1, %3 ]
  %13 = getelementptr inbounds [3 x i32], [3 x i32]* %4, i64 0, i64 %12
  %14 = load volatile i32, i32* %13, align 4, !tbaa !3
  call void @llvm.lifetime.end.p0i8(i64 12, i8* nonnull %5) #6
  ret i32 %14
}

; Function Attrs: nounwind ssp uwtable
define i32 @notct_falsepath(i32, i32, i32) local_unnamed_addr #1 {
  %4 = alloca [3 x i32], align 4
  %5 = bitcast [3 x i32]* %4 to i8*
  call void @llvm.lifetime.start.p0i8(i64 12, i8* nonnull %5) #6
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* nonnull align 4 %5, i8* align 4 bitcast ([3 x i32]* @__const.two_ct_violations.z to i8*), i64 12, i1 true)
  %6 = getelementptr inbounds [3 x i32], [3 x i32]* %4, i64 0, i64 2
  store volatile i32 %1, i32* %6, align 4, !tbaa !3
  %7 = icmp sgt i32 %2, 3
  br i1 %7, label %11, label %8

8:                                                ; preds = %3
  %9 = srem i32 %0, 3
  %10 = sext i32 %9 to i64
  br label %11

11:                                               ; preds = %3, %8
  %12 = phi i64 [ %10, %8 ], [ 1, %3 ]
  %13 = getelementptr inbounds [3 x i32], [3 x i32]* %4, i64 0, i64 %12
  %14 = load volatile i32, i32* %13, align 4, !tbaa !3
  call void @llvm.lifetime.end.p0i8(i64 12, i8* nonnull %5) #6
  ret i32 %14
}

; Function Attrs: nounwind ssp uwtable
define i32 @two_ct_violations(i32, i32, i32) local_unnamed_addr #1 {
  %4 = alloca [3 x i32], align 4
  %5 = bitcast [3 x i32]* %4 to i8*
  call void @llvm.lifetime.start.p0i8(i64 12, i8* nonnull %5) #6
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* nonnull align 4 %5, i8* align 4 bitcast ([3 x i32]* @__const.two_ct_violations.z to i8*), i64 12, i1 true)
  %6 = getelementptr inbounds [3 x i32], [3 x i32]* %4, i64 0, i64 2
  store volatile i32 %1, i32* %6, align 4, !tbaa !3
  %7 = icmp slt i32 %2, 3
  br i1 %7, label %8, label %11

8:                                                ; preds = %3
  %9 = srem i32 %0, 3
  %10 = sext i32 %9 to i64
  br label %16

11:                                               ; preds = %3
  %12 = icmp sgt i32 %2, 100
  br i1 %12, label %16, label %13

13:                                               ; preds = %11
  %14 = add nsw i32 %1, -2
  %15 = sext i32 %14 to i64
  br label %16

16:                                               ; preds = %11, %13, %8
  %17 = phi i64 [ %15, %13 ], [ %10, %8 ], [ 0, %11 ]
  %18 = getelementptr inbounds [3 x i32], [3 x i32]* %4, i64 0, i64 %17
  %19 = load volatile i32, i32* %18, align 4, !tbaa !3
  call void @llvm.lifetime.end.p0i8(i64 12, i8* nonnull %5) #6
  ret i32 %19
}

; Function Attrs: norecurse nounwind readnone ssp uwtable
define i32 @ct_onearg(i32, i32) local_unnamed_addr #0 {
  %3 = icmp sgt i32 %0, 100
  br i1 %3, label %7, label %4

4:                                                ; preds = %2
  %5 = srem i32 %0, 20
  %6 = mul nsw i32 %5, 3
  br label %7

7:                                                ; preds = %2, %4
  %8 = phi i32 [ %6, %4 ], [ %1, %2 ]
  ret i32 %8
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @ct_secrets(i32* nocapture readonly) local_unnamed_addr #3 {
  %2 = getelementptr inbounds i32, i32* %0, i64 20
  %3 = load i32, i32* %2, align 4, !tbaa !3
  %4 = add nsw i32 %3, 3
  ret i32 %4
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @notct_secrets(i32* nocapture readonly) local_unnamed_addr #3 {
  %2 = getelementptr inbounds i32, i32* %0, i64 20
  %3 = load i32, i32* %2, align 4, !tbaa !3
  %4 = icmp sgt i32 %3, 3
  br i1 %4, label %5, label %8

5:                                                ; preds = %1
  %6 = load i32, i32* %0, align 4, !tbaa !3
  %7 = mul nsw i32 %6, 3
  br label %12

8:                                                ; preds = %1
  %9 = getelementptr inbounds i32, i32* %0, i64 2
  %10 = load i32, i32* %9, align 4, !tbaa !3
  %11 = sdiv i32 %10, 22
  br label %12

12:                                               ; preds = %8, %5
  %13 = phi i32 [ %7, %5 ], [ %11, %8 ]
  ret i32 %13
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @ct_struct(i32* nocapture readonly, %struct.PartiallySecret* nocapture readonly) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %1, i64 0, i32 0
  %4 = load i32, i32* %3, align 4, !tbaa !7
  %5 = sext i32 %4 to i64
  %6 = getelementptr inbounds i32, i32* %0, i64 %5
  %7 = load i32, i32* %6, align 4, !tbaa !3
  %8 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %1, i64 0, i32 1
  %9 = load i32, i32* %8, align 4, !tbaa !9
  %10 = add nsw i32 %9, %7
  ret i32 %10
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @notct_struct(i32* nocapture readonly, %struct.PartiallySecret* nocapture readonly) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %1, i64 0, i32 1
  %4 = load i32, i32* %3, align 4, !tbaa !9
  %5 = sext i32 %4 to i64
  %6 = getelementptr inbounds i32, i32* %0, i64 %5
  %7 = load i32, i32* %6, align 4, !tbaa !3
  %8 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %1, i64 0, i32 0
  %9 = load i32, i32* %8, align 4, !tbaa !7
  %10 = add nsw i32 %9, %7
  ret i32 %10
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @notct_maybenull_null(i32* nocapture readonly, i32* readnone, %struct.PartiallySecret* nocapture readonly) local_unnamed_addr #3 {
  %4 = icmp eq i32* %1, null
  %5 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %2, i64 0, i32 0
  %6 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %2, i64 0, i32 1
  %7 = select i1 %4, i32* %6, i32* %5
  %8 = load i32, i32* %7, align 4, !tbaa !3
  %9 = sext i32 %8 to i64
  %10 = getelementptr inbounds i32, i32* %0, i64 %9
  %11 = load i32, i32* %10, align 4, !tbaa !3
  ret i32 %11
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @notct_maybenull_notnull(i32* nocapture readonly, i32* readonly, %struct.PartiallySecret* nocapture readonly) local_unnamed_addr #3 {
  %4 = icmp eq i32* %1, null
  %5 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %2, i64 0, i32 0
  %6 = getelementptr inbounds %struct.PartiallySecret, %struct.PartiallySecret* %2, i64 0, i32 1
  %7 = select i1 %4, i32* %5, i32* %6
  %8 = select i1 %4, i32* %0, i32* %1
  %9 = load i32, i32* %7, align 4, !tbaa !3
  %10 = sext i32 %9 to i64
  %11 = getelementptr inbounds i32, i32* %8, i64 %10
  %12 = load i32, i32* %11, align 4, !tbaa !3
  ret i32 %12
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @ct_doubleptr(i32** nocapture readonly) local_unnamed_addr #3 {
  %2 = getelementptr inbounds i32*, i32** %0, i64 2
  %3 = load i32*, i32** %2, align 8, !tbaa !10
  %4 = getelementptr inbounds i32, i32* %3, i64 5
  %5 = load i32, i32* %4, align 4, !tbaa !3
  %6 = add nsw i32 %5, 3
  ret i32 %6
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @notct_doubleptr(i32** nocapture readonly) local_unnamed_addr #3 {
  %2 = getelementptr inbounds i32*, i32** %0, i64 2
  %3 = load i32*, i32** %2, align 8, !tbaa !10
  %4 = getelementptr inbounds i32, i32* %3, i64 5
  %5 = load i32, i32* %4, align 4, !tbaa !3
  %6 = icmp sgt i32 %5, 3
  br i1 %6, label %7, label %12

7:                                                ; preds = %1
  %8 = load i32*, i32** %0, align 8, !tbaa !10
  %9 = getelementptr inbounds i32, i32* %8, i64 10
  %10 = load i32, i32* %9, align 4, !tbaa !3
  %11 = mul nsw i32 %10, 3
  br label %16

12:                                               ; preds = %1
  %13 = getelementptr inbounds i32, i32* %3, i64 22
  %14 = load i32, i32* %13, align 4, !tbaa !3
  %15 = sdiv i32 %14, 5
  br label %16

16:                                               ; preds = %12, %7
  %17 = phi i32 [ %11, %7 ], [ %15, %12 ]
  ret i32 %17
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @ct_struct_voidptr(i32* nocapture readonly, i8* nocapture readonly) local_unnamed_addr #3 {
  %3 = bitcast i8* %1 to i32*
  %4 = load i32, i32* %3, align 4, !tbaa !7
  %5 = sext i32 %4 to i64
  %6 = getelementptr inbounds i32, i32* %0, i64 %5
  %7 = load i32, i32* %6, align 4, !tbaa !3
  %8 = getelementptr inbounds i8, i8* %1, i64 4
  %9 = bitcast i8* %8 to i32*
  %10 = load i32, i32* %9, align 4, !tbaa !9
  %11 = add nsw i32 %10, %7
  ret i32 %11
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @notct_struct_voidptr(i32* nocapture readonly, i8* nocapture readonly) local_unnamed_addr #3 {
  %3 = getelementptr inbounds i8, i8* %1, i64 4
  %4 = bitcast i8* %3 to i32*
  %5 = load i32, i32* %4, align 4, !tbaa !9
  %6 = sext i32 %5 to i64
  %7 = getelementptr inbounds i32, i32* %0, i64 %6
  %8 = load i32, i32* %7, align 4, !tbaa !3
  %9 = bitcast i8* %1 to i32*
  %10 = load i32, i32* %9, align 4, !tbaa !7
  %11 = add nsw i32 %10, %8
  ret i32 %11
}

; Function Attrs: norecurse nounwind readonly ssp uwtable
define i32 @indirectly_recursive_struct(i32* nocapture readonly, %struct.Parent* nocapture readonly) local_unnamed_addr #3 {
  %3 = getelementptr inbounds %struct.Parent, %struct.Parent* %1, i64 0, i32 2
  %4 = load %struct.Child*, %struct.Child** %3, align 8, !tbaa !12
  %5 = getelementptr inbounds %struct.Child, %struct.Child* %4, i64 0, i32 1
  %6 = load %struct.Parent*, %struct.Parent** %5, align 8, !tbaa !14
  %7 = getelementptr inbounds %struct.Parent, %struct.Parent* %6, i64 0, i32 0
  %8 = load i32, i32* %7, align 8, !tbaa !16
  %9 = sext i32 %8 to i64
  %10 = getelementptr inbounds i32, i32* %0, i64 %9
  %11 = load i32, i32* %10, align 4, !tbaa !3
  ret i32 %11
}

; Function Attrs: nounwind readnone ssp uwtable
define i32 @related_args(i32, i32, i32) local_unnamed_addr #4 {
  %4 = alloca [20 x i32], align 16
  %5 = bitcast [20 x i32]* %4 to i8*
  call void @llvm.lifetime.start.p0i8(i64 80, i8* nonnull %5) #6
  %6 = icmp ult i32 %0, 20
  br i1 %6, label %7, label %82

7:                                                ; preds = %3
  %8 = zext i32 %0 to i64
  %9 = sub nsw i64 20, %8
  %10 = icmp ult i64 %9, 8
  br i1 %10, label %11, label %13

11:                                               ; preds = %70, %7
  %12 = phi i64 [ %8, %7 ], [ %15, %70 ]
  br label %77

13:                                               ; preds = %7
  %14 = and i64 %9, -8
  %15 = add nsw i64 %14, %8
  %16 = insertelement <4 x i32> undef, i32 %2, i32 0
  %17 = shufflevector <4 x i32> %16, <4 x i32> undef, <4 x i32> zeroinitializer
  %18 = insertelement <4 x i32> undef, i32 %2, i32 0
  %19 = shufflevector <4 x i32> %18, <4 x i32> undef, <4 x i32> zeroinitializer
  %20 = add nsw i64 %14, -8
  %21 = lshr exact i64 %20, 3
  %22 = add nuw nsw i64 %21, 1
  %23 = and i64 %22, 3
  %24 = icmp ult i64 %20, 24
  br i1 %24, label %56, label %25

25:                                               ; preds = %13
  %26 = sub nsw i64 %22, %23
  br label %27

27:                                               ; preds = %27, %25
  %28 = phi i64 [ 0, %25 ], [ %53, %27 ]
  %29 = phi i64 [ %26, %25 ], [ %54, %27 ]
  %30 = add i64 %28, %8
  %31 = getelementptr inbounds [20 x i32], [20 x i32]* %4, i64 0, i64 %30
  %32 = bitcast i32* %31 to <4 x i32>*
  store <4 x i32> %17, <4 x i32>* %32, align 4, !tbaa !3
  %33 = getelementptr inbounds i32, i32* %31, i64 4
  %34 = bitcast i32* %33 to <4 x i32>*
  store <4 x i32> %19, <4 x i32>* %34, align 4, !tbaa !3
  %35 = or i64 %28, 8
  %36 = add i64 %35, %8
  %37 = getelementptr inbounds [20 x i32], [20 x i32]* %4, i64 0, i64 %36
  %38 = bitcast i32* %37 to <4 x i32>*
  store <4 x i32> %17, <4 x i32>* %38, align 4, !tbaa !3
  %39 = getelementptr inbounds i32, i32* %37, i64 4
  %40 = bitcast i32* %39 to <4 x i32>*
  store <4 x i32> %19, <4 x i32>* %40, align 4, !tbaa !3
  %41 = or i64 %28, 16
  %42 = add i64 %41, %8
  %43 = getelementptr inbounds [20 x i32], [20 x i32]* %4, i64 0, i64 %42
  %44 = bitcast i32* %43 to <4 x i32>*
  store <4 x i32> %17, <4 x i32>* %44, align 4, !tbaa !3
  %45 = getelementptr inbounds i32, i32* %43, i64 4
  %46 = bitcast i32* %45 to <4 x i32>*
  store <4 x i32> %19, <4 x i32>* %46, align 4, !tbaa !3
  %47 = or i64 %28, 24
  %48 = add i64 %47, %8
  %49 = getelementptr inbounds [20 x i32], [20 x i32]* %4, i64 0, i64 %48
  %50 = bitcast i32* %49 to <4 x i32>*
  store <4 x i32> %17, <4 x i32>* %50, align 4, !tbaa !3
  %51 = getelementptr inbounds i32, i32* %49, i64 4
  %52 = bitcast i32* %51 to <4 x i32>*
  store <4 x i32> %19, <4 x i32>* %52, align 4, !tbaa !3
  %53 = add i64 %28, 32
  %54 = add i64 %29, -4
  %55 = icmp eq i64 %54, 0
  br i1 %55, label %56, label %27, !llvm.loop !17

56:                                               ; preds = %27, %13
  %57 = phi i64 [ 0, %13 ], [ %53, %27 ]
  %58 = icmp eq i64 %23, 0
  br i1 %58, label %70, label %59

59:                                               ; preds = %56, %59
  %60 = phi i64 [ %67, %59 ], [ %57, %56 ]
  %61 = phi i64 [ %68, %59 ], [ %23, %56 ]
  %62 = add i64 %60, %8
  %63 = getelementptr inbounds [20 x i32], [20 x i32]* %4, i64 0, i64 %62
  %64 = bitcast i32* %63 to <4 x i32>*
  store <4 x i32> %17, <4 x i32>* %64, align 4, !tbaa !3
  %65 = getelementptr inbounds i32, i32* %63, i64 4
  %66 = bitcast i32* %65 to <4 x i32>*
  store <4 x i32> %19, <4 x i32>* %66, align 4, !tbaa !3
  %67 = add i64 %60, 8
  %68 = add i64 %61, -1
  %69 = icmp eq i64 %68, 0
  br i1 %69, label %70, label %59, !llvm.loop !19

70:                                               ; preds = %59, %56
  %71 = icmp eq i64 %9, %14
  br i1 %71, label %72, label %11

72:                                               ; preds = %77, %70
  %73 = zext i32 %1 to i64
  %74 = getelementptr inbounds [20 x i32], [20 x i32]* %4, i64 0, i64 %73
  %75 = load i32, i32* %74, align 4, !tbaa !3
  %76 = icmp eq i32 %75, 0
  br i1 %76, label %88, label %82

77:                                               ; preds = %11, %77
  %78 = phi i64 [ %80, %77 ], [ %12, %11 ]
  %79 = getelementptr inbounds [20 x i32], [20 x i32]* %4, i64 0, i64 %78
  store i32 %2, i32* %79, align 4, !tbaa !3
  %80 = add nuw nsw i64 %78, 1
  %81 = icmp eq i64 %80, 20
  br i1 %81, label %72, label %77, !llvm.loop !21

82:                                               ; preds = %3, %72
  %83 = getelementptr inbounds [20 x i32], [20 x i32]* %4, i64 0, i64 0
  %84 = load i32, i32* %83, align 16, !tbaa !3
  %85 = mul nsw i32 %84, 33
  %86 = add i32 %1, %0
  %87 = add i32 %86, %85
  br label %88

88:                                               ; preds = %72, %82
  %89 = phi i32 [ %87, %82 ], [ 1, %72 ]
  call void @llvm.lifetime.end.p0i8(i64 80, i8* nonnull %5) #6
  ret i32 %89
}

; Function Attrs: nounwind readonly ssp uwtable
define i32 @struct_related_fields(%struct.StructWithRelatedFields* nocapture readonly) local_unnamed_addr #5 {
  %2 = alloca [20 x i32], align 16
  %3 = bitcast [20 x i32]* %2 to i8*
  call void @llvm.lifetime.start.p0i8(i64 80, i8* nonnull %3) #6
  %4 = getelementptr inbounds %struct.StructWithRelatedFields, %struct.StructWithRelatedFields* %0, i64 0, i32 0
  %5 = load i32, i32* %4, align 4, !tbaa !23
  %6 = icmp ult i32 %5, 20
  br i1 %6, label %7, label %74

7:                                                ; preds = %1
  %8 = getelementptr inbounds %struct.StructWithRelatedFields, %struct.StructWithRelatedFields* %0, i64 0, i32 2
  %9 = load i32, i32* %8, align 4, !tbaa !25
  %10 = zext i32 %5 to i64
  %11 = sub nsw i64 20, %10
  %12 = icmp ult i64 %11, 8
  br i1 %12, label %13, label %15

13:                                               ; preds = %72, %7
  %14 = phi i64 [ %10, %7 ], [ %17, %72 ]
  br label %81

15:                                               ; preds = %7
  %16 = and i64 %11, -8
  %17 = add nsw i64 %16, %10
  %18 = insertelement <4 x i32> undef, i32 %9, i32 0
  %19 = shufflevector <4 x i32> %18, <4 x i32> undef, <4 x i32> zeroinitializer
  %20 = insertelement <4 x i32> undef, i32 %9, i32 0
  %21 = shufflevector <4 x i32> %20, <4 x i32> undef, <4 x i32> zeroinitializer
  %22 = add nsw i64 %16, -8
  %23 = lshr exact i64 %22, 3
  %24 = add nuw nsw i64 %23, 1
  %25 = and i64 %24, 3
  %26 = icmp ult i64 %22, 24
  br i1 %26, label %58, label %27

27:                                               ; preds = %15
  %28 = sub nsw i64 %24, %25
  br label %29

29:                                               ; preds = %29, %27
  %30 = phi i64 [ 0, %27 ], [ %55, %29 ]
  %31 = phi i64 [ %28, %27 ], [ %56, %29 ]
  %32 = add i64 %30, %10
  %33 = getelementptr inbounds [20 x i32], [20 x i32]* %2, i64 0, i64 %32
  %34 = bitcast i32* %33 to <4 x i32>*
  store <4 x i32> %19, <4 x i32>* %34, align 4, !tbaa !3
  %35 = getelementptr inbounds i32, i32* %33, i64 4
  %36 = bitcast i32* %35 to <4 x i32>*
  store <4 x i32> %21, <4 x i32>* %36, align 4, !tbaa !3
  %37 = or i64 %30, 8
  %38 = add i64 %37, %10
  %39 = getelementptr inbounds [20 x i32], [20 x i32]* %2, i64 0, i64 %38
  %40 = bitcast i32* %39 to <4 x i32>*
  store <4 x i32> %19, <4 x i32>* %40, align 4, !tbaa !3
  %41 = getelementptr inbounds i32, i32* %39, i64 4
  %42 = bitcast i32* %41 to <4 x i32>*
  store <4 x i32> %21, <4 x i32>* %42, align 4, !tbaa !3
  %43 = or i64 %30, 16
  %44 = add i64 %43, %10
  %45 = getelementptr inbounds [20 x i32], [20 x i32]* %2, i64 0, i64 %44
  %46 = bitcast i32* %45 to <4 x i32>*
  store <4 x i32> %19, <4 x i32>* %46, align 4, !tbaa !3
  %47 = getelementptr inbounds i32, i32* %45, i64 4
  %48 = bitcast i32* %47 to <4 x i32>*
  store <4 x i32> %21, <4 x i32>* %48, align 4, !tbaa !3
  %49 = or i64 %30, 24
  %50 = add i64 %49, %10
  %51 = getelementptr inbounds [20 x i32], [20 x i32]* %2, i64 0, i64 %50
  %52 = bitcast i32* %51 to <4 x i32>*
  store <4 x i32> %19, <4 x i32>* %52, align 4, !tbaa !3
  %53 = getelementptr inbounds i32, i32* %51, i64 4
  %54 = bitcast i32* %53 to <4 x i32>*
  store <4 x i32> %21, <4 x i32>* %54, align 4, !tbaa !3
  %55 = add i64 %30, 32
  %56 = add i64 %31, -4
  %57 = icmp eq i64 %56, 0
  br i1 %57, label %58, label %29, !llvm.loop !26

58:                                               ; preds = %29, %15
  %59 = phi i64 [ 0, %15 ], [ %55, %29 ]
  %60 = icmp eq i64 %25, 0
  br i1 %60, label %72, label %61

61:                                               ; preds = %58, %61
  %62 = phi i64 [ %69, %61 ], [ %59, %58 ]
  %63 = phi i64 [ %70, %61 ], [ %25, %58 ]
  %64 = add i64 %62, %10
  %65 = getelementptr inbounds [20 x i32], [20 x i32]* %2, i64 0, i64 %64
  %66 = bitcast i32* %65 to <4 x i32>*
  store <4 x i32> %19, <4 x i32>* %66, align 4, !tbaa !3
  %67 = getelementptr inbounds i32, i32* %65, i64 4
  %68 = bitcast i32* %67 to <4 x i32>*
  store <4 x i32> %21, <4 x i32>* %68, align 4, !tbaa !3
  %69 = add i64 %62, 8
  %70 = add i64 %63, -1
  %71 = icmp eq i64 %70, 0
  br i1 %71, label %72, label %61, !llvm.loop !27

72:                                               ; preds = %61, %58
  %73 = icmp eq i64 %11, %16
  br i1 %73, label %74, label %13

74:                                               ; preds = %81, %72, %1
  %75 = getelementptr inbounds %struct.StructWithRelatedFields, %struct.StructWithRelatedFields* %0, i64 0, i32 1
  %76 = load i32, i32* %75, align 4, !tbaa !28
  %77 = zext i32 %76 to i64
  %78 = getelementptr inbounds [20 x i32], [20 x i32]* %2, i64 0, i64 %77
  %79 = load i32, i32* %78, align 4, !tbaa !3
  %80 = icmp eq i32 %79, 0
  br i1 %80, label %92, label %86

81:                                               ; preds = %13, %81
  %82 = phi i64 [ %84, %81 ], [ %14, %13 ]
  %83 = getelementptr inbounds [20 x i32], [20 x i32]* %2, i64 0, i64 %82
  store i32 %9, i32* %83, align 4, !tbaa !3
  %84 = add nuw nsw i64 %82, 1
  %85 = icmp eq i64 %84, 20
  br i1 %85, label %74, label %81, !llvm.loop !29

86:                                               ; preds = %74
  %87 = getelementptr inbounds [20 x i32], [20 x i32]* %2, i64 0, i64 0
  %88 = load i32, i32* %87, align 16, !tbaa !3
  %89 = mul nsw i32 %88, 33
  %90 = add i32 %76, %5
  %91 = add i32 %90, %89
  br label %92

92:                                               ; preds = %74, %86
  %93 = phi i32 [ %91, %86 ], [ 1, %74 ]
  call void @llvm.lifetime.end.p0i8(i64 80, i8* nonnull %3) #6
  ret i32 %93
}

attributes #0 = { norecurse nounwind readnone ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+cx8,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #1 = { nounwind ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+cx8,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #2 = { argmemonly nounwind }
attributes #3 = { norecurse nounwind readonly ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+cx8,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #4 = { nounwind readnone ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+cx8,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #5 = { nounwind readonly ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+cx8,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #6 = { nounwind }

!llvm.module.flags = !{!0, !1}
!llvm.ident = !{!2}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 7, !"PIC Level", i32 2}
!2 = !{!"clang version 9.0.0 (tags/RELEASE_900/final)"}
!3 = !{!4, !4, i64 0}
!4 = !{!"int", !5, i64 0}
!5 = !{!"omnipotent char", !6, i64 0}
!6 = !{!"Simple C/C++ TBAA"}
!7 = !{!8, !4, i64 0}
!8 = !{!"PartiallySecret", !4, i64 0, !4, i64 4}
!9 = !{!8, !4, i64 4}
!10 = !{!11, !11, i64 0}
!11 = !{!"any pointer", !5, i64 0}
!12 = !{!13, !11, i64 16}
!13 = !{!"Parent", !4, i64 0, !11, i64 8, !11, i64 16}
!14 = !{!15, !11, i64 8}
!15 = !{!"Child", !4, i64 0, !11, i64 8}
!16 = !{!13, !4, i64 0}
!17 = distinct !{!17, !18}
!18 = !{!"llvm.loop.isvectorized", i32 1}
!19 = distinct !{!19, !20}
!20 = !{!"llvm.loop.unroll.disable"}
!21 = distinct !{!21, !22, !18}
!22 = !{!"llvm.loop.unroll.runtime.disable"}
!23 = !{!24, !4, i64 0}
!24 = !{!"StructWithRelatedFields", !4, i64 0, !4, i64 4, !4, i64 8}
!25 = !{!24, !4, i64 8}
!26 = distinct !{!26, !18}
!27 = distinct !{!27, !20}
!28 = !{!24, !4, i64 4}
!29 = distinct !{!29, !22, !18}
