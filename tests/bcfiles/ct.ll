; ModuleID = 'ct.c'
source_filename = "ct.c"
target datalayout = "e-m:o-i64:64-f80:128-n8:16:32:64-S128"
target triple = "x86_64-apple-macosx10.14.0"

%struct.PartiallySecret = type { i32, i32 }
%struct.Parent = type { i32, %struct.Child*, %struct.Child* }
%struct.Child = type { i32, %struct.Parent* }
%struct.StructWithRelatedFields = type { i32, i32, i32 }

@__const.notct_onepath.z = private unnamed_addr constant [3 x i32] [i32 0, i32 2, i32 300], align 4

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

; <label>:7:                                      ; preds = %2
  %8 = mul nsw i32 %0, 5
  br label %11

; <label>:9:                                      ; preds = %2
  %10 = sdiv i32 %1, 99
  br label %11

; <label>:11:                                     ; preds = %9, %7
  %12 = phi i32 [ %8, %7 ], [ %10, %9 ]
  call void @llvm.lifetime.end.p0i8(i64 4, i8* nonnull %4)
  ret i32 %12
}

; Function Attrs: argmemonly nounwind
declare void @llvm.lifetime.start.p0i8(i64, i8* nocapture) #2

; Function Attrs: argmemonly nounwind
declare void @llvm.lifetime.end.p0i8(i64, i8* nocapture) #2

; Function Attrs: norecurse nounwind readnone ssp uwtable
define i32 @notct_branch(i32) local_unnamed_addr #0 {
  %2 = icmp sgt i32 %0, 10
  br i1 %2, label %3, label %6

; <label>:3:                                      ; preds = %1
  %4 = urem i32 %0, 200
  %5 = mul nuw nsw i32 %4, 3
  br label %8

; <label>:6:                                      ; preds = %1
  %7 = add nsw i32 %0, 10
  br label %8

; <label>:8:                                      ; preds = %6, %3
  %9 = phi i32 [ %5, %3 ], [ %7, %6 ]
  ret i32 %9
}

; Function Attrs: nounwind ssp uwtable
define i32 @notct_mem(i32) local_unnamed_addr #1 {
  %2 = alloca [3 x i32], align 4
  %3 = bitcast [3 x i32]* %2 to i8*
  call void @llvm.lifetime.start.p0i8(i64 12, i8* nonnull %3) #6
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* nonnull align 4 %3, i8* align 4 bitcast ([3 x i32]* @__const.notct_onepath.z to i8*), i64 12, i1 true)
  %4 = srem i32 %0, 3
  %5 = sext i32 %4 to i64
  %6 = getelementptr inbounds [3 x i32], [3 x i32]* %2, i64 0, i64 %5
  %7 = load volatile i32, i32* %6, align 4, !tbaa !3
  call void @llvm.lifetime.end.p0i8(i64 12, i8* nonnull %3) #6
  ret i32 %7
}

; Function Attrs: argmemonly nounwind
declare void @llvm.memcpy.p0i8.p0i8.i64(i8* nocapture writeonly, i8* nocapture readonly, i64, i1) #2

; Function Attrs: nounwind ssp uwtable
define i32 @notct_onepath(i32, i32) local_unnamed_addr #1 {
  %3 = alloca [3 x i32], align 4
  %4 = bitcast [3 x i32]* %3 to i8*
  call void @llvm.lifetime.start.p0i8(i64 12, i8* nonnull %4) #6
  call void @llvm.memcpy.p0i8.p0i8.i64(i8* nonnull align 4 %4, i8* align 4 bitcast ([3 x i32]* @__const.notct_onepath.z to i8*), i64 12, i1 true)
  %5 = getelementptr inbounds [3 x i32], [3 x i32]* %3, i64 0, i64 2
  store volatile i32 %1, i32* %5, align 4, !tbaa !3
  %6 = load volatile i32, i32* %5, align 4, !tbaa !3
  %7 = icmp sgt i32 %6, 3
  br i1 %7, label %8, label %11

; <label>:8:                                      ; preds = %2
  %9 = srem i32 %0, 3
  %10 = sext i32 %9 to i64
  br label %11

; <label>:11:                                     ; preds = %2, %8
  %12 = phi i64 [ %10, %8 ], [ 1, %2 ]
  %13 = getelementptr inbounds [3 x i32], [3 x i32]* %3, i64 0, i64 %12
  %14 = load volatile i32, i32* %13, align 4, !tbaa !3
  call void @llvm.lifetime.end.p0i8(i64 12, i8* nonnull %4) #6
  ret i32 %14
}

; Function Attrs: norecurse nounwind readnone ssp uwtable
define i32 @ct_onearg(i32, i32) local_unnamed_addr #0 {
  %3 = icmp sgt i32 %0, 100
  br i1 %3, label %7, label %4

; <label>:4:                                      ; preds = %2
  %5 = srem i32 %0, 20
  %6 = mul nsw i32 %5, 3
  br label %7

; <label>:7:                                      ; preds = %2, %4
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

; <label>:5:                                      ; preds = %1
  %6 = load i32, i32* %0, align 4, !tbaa !3
  %7 = mul nsw i32 %6, 3
  br label %12

; <label>:8:                                      ; preds = %1
  %9 = getelementptr inbounds i32, i32* %0, i64 2
  %10 = load i32, i32* %9, align 4, !tbaa !3
  %11 = sdiv i32 %10, 22
  br label %12

; <label>:12:                                     ; preds = %8, %5
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

; <label>:7:                                      ; preds = %1
  %8 = load i32*, i32** %0, align 8, !tbaa !10
  %9 = getelementptr inbounds i32, i32* %8, i64 10
  %10 = load i32, i32* %9, align 4, !tbaa !3
  %11 = mul nsw i32 %10, 3
  br label %16

; <label>:12:                                     ; preds = %1
  %13 = getelementptr inbounds i32, i32* %3, i64 22
  %14 = load i32, i32* %13, align 4, !tbaa !3
  %15 = sdiv i32 %14, 5
  br label %16

; <label>:16:                                     ; preds = %12, %7
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
  %4 = alloca [100 x i32], align 16
  %5 = bitcast [100 x i32]* %4 to i8*
  call void @llvm.lifetime.start.p0i8(i64 400, i8* nonnull %5) #6
  %6 = icmp ult i32 %0, 100
  br i1 %6, label %7, label %44

; <label>:7:                                      ; preds = %3
  %8 = zext i32 %0 to i64
  %9 = sub nsw i64 100, %8
  %10 = icmp ult i64 %9, 8
  br i1 %10, label %11, label %13

; <label>:11:                                     ; preds = %32, %7
  %12 = phi i64 [ %8, %7 ], [ %18, %32 ]
  br label %39

; <label>:13:                                     ; preds = %7
  %14 = sub i32 4, %0
  %15 = and i32 %14, 7
  %16 = zext i32 %15 to i64
  %17 = sub nsw i64 %9, %16
  %18 = add nsw i64 %17, %8
  %19 = insertelement <4 x i32> undef, i32 %2, i32 0
  %20 = shufflevector <4 x i32> %19, <4 x i32> undef, <4 x i32> zeroinitializer
  %21 = insertelement <4 x i32> undef, i32 %2, i32 0
  %22 = shufflevector <4 x i32> %21, <4 x i32> undef, <4 x i32> zeroinitializer
  br label %23

; <label>:23:                                     ; preds = %23, %13
  %24 = phi i64 [ 0, %13 ], [ %30, %23 ]
  %25 = add i64 %24, %8
  %26 = getelementptr inbounds [100 x i32], [100 x i32]* %4, i64 0, i64 %25
  %27 = bitcast i32* %26 to <4 x i32>*
  store <4 x i32> %20, <4 x i32>* %27, align 4, !tbaa !3
  %28 = getelementptr inbounds i32, i32* %26, i64 4
  %29 = bitcast i32* %28 to <4 x i32>*
  store <4 x i32> %22, <4 x i32>* %29, align 4, !tbaa !3
  %30 = add i64 %24, 8
  %31 = icmp eq i64 %30, %17
  br i1 %31, label %32, label %23, !llvm.loop !17

; <label>:32:                                     ; preds = %23
  %33 = icmp eq i32 %15, 0
  br i1 %33, label %34, label %11

; <label>:34:                                     ; preds = %39, %32
  %35 = zext i32 %1 to i64
  %36 = getelementptr inbounds [100 x i32], [100 x i32]* %4, i64 0, i64 %35
  %37 = load i32, i32* %36, align 4, !tbaa !3
  %38 = icmp eq i32 %37, 0
  br i1 %38, label %50, label %44

; <label>:39:                                     ; preds = %11, %39
  %40 = phi i64 [ %42, %39 ], [ %12, %11 ]
  %41 = getelementptr inbounds [100 x i32], [100 x i32]* %4, i64 0, i64 %40
  store i32 %2, i32* %41, align 4, !tbaa !3
  %42 = add nuw nsw i64 %40, 1
  %43 = icmp eq i64 %42, 100
  br i1 %43, label %34, label %39, !llvm.loop !19

; <label>:44:                                     ; preds = %3, %34
  %45 = getelementptr inbounds [100 x i32], [100 x i32]* %4, i64 0, i64 0
  %46 = load i32, i32* %45, align 16, !tbaa !3
  %47 = mul nsw i32 %46, 33
  %48 = add i32 %1, %0
  %49 = add i32 %48, %47
  br label %50

; <label>:50:                                     ; preds = %34, %44
  %51 = phi i32 [ %49, %44 ], [ 1, %34 ]
  call void @llvm.lifetime.end.p0i8(i64 400, i8* nonnull %5) #6
  ret i32 %51
}

; Function Attrs: nounwind readonly ssp uwtable
define i32 @struct_related_fields(%struct.StructWithRelatedFields* nocapture readonly) local_unnamed_addr #5 {
  %2 = alloca [100 x i32], align 16
  %3 = bitcast [100 x i32]* %2 to i8*
  call void @llvm.lifetime.start.p0i8(i64 400, i8* nonnull %3) #6
  %4 = getelementptr inbounds %struct.StructWithRelatedFields, %struct.StructWithRelatedFields* %0, i64 0, i32 0
  %5 = load i32, i32* %4, align 4, !tbaa !21
  %6 = icmp ult i32 %5, 100
  br i1 %6, label %7, label %36

; <label>:7:                                      ; preds = %1
  %8 = getelementptr inbounds %struct.StructWithRelatedFields, %struct.StructWithRelatedFields* %0, i64 0, i32 2
  %9 = load i32, i32* %8, align 4, !tbaa !23
  %10 = zext i32 %5 to i64
  %11 = sub nsw i64 100, %10
  %12 = icmp ult i64 %11, 8
  br i1 %12, label %13, label %15

; <label>:13:                                     ; preds = %34, %7
  %14 = phi i64 [ %10, %7 ], [ %20, %34 ]
  br label %43

; <label>:15:                                     ; preds = %7
  %16 = sub i32 4, %5
  %17 = and i32 %16, 7
  %18 = zext i32 %17 to i64
  %19 = sub nsw i64 %11, %18
  %20 = add nsw i64 %19, %10
  %21 = insertelement <4 x i32> undef, i32 %9, i32 0
  %22 = shufflevector <4 x i32> %21, <4 x i32> undef, <4 x i32> zeroinitializer
  %23 = insertelement <4 x i32> undef, i32 %9, i32 0
  %24 = shufflevector <4 x i32> %23, <4 x i32> undef, <4 x i32> zeroinitializer
  br label %25

; <label>:25:                                     ; preds = %25, %15
  %26 = phi i64 [ 0, %15 ], [ %32, %25 ]
  %27 = add i64 %26, %10
  %28 = getelementptr inbounds [100 x i32], [100 x i32]* %2, i64 0, i64 %27
  %29 = bitcast i32* %28 to <4 x i32>*
  store <4 x i32> %22, <4 x i32>* %29, align 4, !tbaa !3
  %30 = getelementptr inbounds i32, i32* %28, i64 4
  %31 = bitcast i32* %30 to <4 x i32>*
  store <4 x i32> %24, <4 x i32>* %31, align 4, !tbaa !3
  %32 = add i64 %26, 8
  %33 = icmp eq i64 %32, %19
  br i1 %33, label %34, label %25, !llvm.loop !24

; <label>:34:                                     ; preds = %25
  %35 = icmp eq i32 %17, 0
  br i1 %35, label %36, label %13

; <label>:36:                                     ; preds = %43, %34, %1
  %37 = getelementptr inbounds %struct.StructWithRelatedFields, %struct.StructWithRelatedFields* %0, i64 0, i32 1
  %38 = load i32, i32* %37, align 4, !tbaa !25
  %39 = zext i32 %38 to i64
  %40 = getelementptr inbounds [100 x i32], [100 x i32]* %2, i64 0, i64 %39
  %41 = load i32, i32* %40, align 4, !tbaa !3
  %42 = icmp eq i32 %41, 0
  br i1 %42, label %54, label %48

; <label>:43:                                     ; preds = %13, %43
  %44 = phi i64 [ %46, %43 ], [ %14, %13 ]
  %45 = getelementptr inbounds [100 x i32], [100 x i32]* %2, i64 0, i64 %44
  store i32 %9, i32* %45, align 4, !tbaa !3
  %46 = add nuw nsw i64 %44, 1
  %47 = icmp eq i64 %46, 100
  br i1 %47, label %36, label %43, !llvm.loop !26

; <label>:48:                                     ; preds = %36
  %49 = getelementptr inbounds [100 x i32], [100 x i32]* %2, i64 0, i64 0
  %50 = load i32, i32* %49, align 16, !tbaa !3
  %51 = mul nsw i32 %50, 33
  %52 = add i32 %38, %5
  %53 = add i32 %52, %51
  br label %54

; <label>:54:                                     ; preds = %36, %48
  %55 = phi i32 [ %53, %48 ], [ 1, %36 ]
  call void @llvm.lifetime.end.p0i8(i64 400, i8* nonnull %3) #6
  ret i32 %55
}

attributes #0 = { norecurse nounwind readnone ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #1 = { nounwind ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #2 = { argmemonly nounwind }
attributes #3 = { norecurse nounwind readonly ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #4 = { nounwind readnone ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #5 = { nounwind readonly ssp uwtable "correctly-rounded-divide-sqrt-fp-math"="false" "disable-tail-calls"="false" "less-precise-fpmad"="false" "min-legal-vector-width"="0" "no-frame-pointer-elim"="true" "no-frame-pointer-elim-non-leaf" "no-infs-fp-math"="false" "no-jump-tables"="false" "no-nans-fp-math"="false" "no-signed-zeros-fp-math"="false" "no-trapping-math"="false" "stack-protector-buffer-size"="8" "target-cpu"="penryn" "target-features"="+cx16,+fxsr,+mmx,+sahf,+sse,+sse2,+sse3,+sse4.1,+ssse3,+x87" "unsafe-fp-math"="false" "use-soft-float"="false" }
attributes #6 = { nounwind }

!llvm.module.flags = !{!0, !1}
!llvm.ident = !{!2}

!0 = !{i32 1, !"wchar_size", i32 4}
!1 = !{i32 7, !"PIC Level", i32 2}
!2 = !{!"clang version 8.0.0 (tags/RELEASE_800/final)"}
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
!19 = distinct !{!19, !20, !18}
!20 = !{!"llvm.loop.unroll.runtime.disable"}
!21 = !{!22, !4, i64 0}
!22 = !{!"StructWithRelatedFields", !4, i64 0, !4, i64 4, !4, i64 8}
!23 = !{!22, !4, i64 8}
!24 = distinct !{!24, !18}
!25 = !{!22, !4, i64 4}
!26 = distinct !{!26, !20, !18}
